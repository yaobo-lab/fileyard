use anyhow::anyhow;
use chrono::Local;
use std::fs::create_dir_all;
use std::sync::OnceLock;
use toolkit_rs::AppResult;
pub fn get_time_stamp() -> i64 {
    Local::now().timestamp()
}
pub fn get_time_stamp_ms() -> i64 {
    Local::now().timestamp_millis()
}

//设备 uuid
static MACHINE_ID: OnceLock<String> = OnceLock::new();
//设备自身 mqtt 消息
static MAC_MQTT_TOPIC: OnceLock<String> = OnceLock::new();

pub fn set_machineid(uuid: String, skuid: &str) -> AppResult {
    //订阅来自 mqtt服务器
    let machine_mqtt_topic = format!("/local/sub/request/{}/{}", skuid, uuid);

    MACHINE_ID
        .set(uuid)
        .map_err(|e| anyhow!("set device uuid error:{}", e))?;

    MAC_MQTT_TOPIC
        .set(machine_mqtt_topic)
        .map_err(|e| anyhow!("set machine topic err:{}", e))?;

    Ok(())
}

pub fn get_machineid() -> &'static str {
    match MACHINE_ID.get() {
        Some(s) => s,
        None => {
            unreachable!("machineid not initialized");
        }
    }
}

pub fn get_machine_topic() -> &'static str {
    match MAC_MQTT_TOPIC.get() {
        Some(s) => s,
        None => {
            unreachable!("machine topic not initialized");
        }
    }
}

pub fn _print_machine_id() {
    match build_machine_id() {
        Ok(id) => log::info!("[machineid] id: {id}"),
        Err(error) => log::warn!("[machineid] unavailable: {error}"),
    }
}

//获取设备指纹
#[allow(unused_assignments)]
pub fn build_machine_id() -> AppResult<String> {
    let output = std::process::Command::new("hostname")
        .output()
        .map_err(|error| anyhow!("machine name is unavailable: {error}"))?;
    let id = String::from_utf8(output.stdout)
        .map_err(|error| anyhow!("machine name is not valid UTF-8: {error}"))?;
    if id.trim().is_empty() {
        return Err(anyhow!("machine name is empty"));
    }
    Ok(blake3::hash(id.as_bytes()).to_hex().to_string())
}

//
pub fn content_as_str(d: &[u8]) -> Option<String> {
    match std::str::from_utf8(d) {
        Ok(s) => Some(s.to_string()),
        Err(_) => {
            log::error!("invalid utf8 index");
            return None;
        }
    }
}

pub fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        // Use PowerShell to set clipboard (handles special characters better than cmd)
        std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Set-Clipboard '{}'", text.replace('\'', "''")),
            ])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(target_os = "macos")]
    {
        use std::io::Write as IoWrite;
        std::process::Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(text.as_bytes());
                }
                child.wait()
            })
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(target_os = "linux")]
    {
        use std::io::Write as IoWrite;
        // Try xclip first, then xsel
        let result = std::process::Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(text.as_bytes());
                }
                child.wait()
            })
            .map(|s| s.success())
            .unwrap_or(false);
        if result {
            return true;
        }
        std::process::Command::new("xsel")
            .args(["--clipboard", "--input"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(text.as_bytes());
                }
                child.wait()
            })
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = text;
        false
    }
}

pub fn open_in_browser(url: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .is_ok()
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn().is_ok()
    }
    #[cfg(target_os = "linux")]
    {
        // Try multiple openers in order. xdg-open is the standard, but it
        // (or the browser it launches) can fail with EPERM in sandboxed
        // environments (containers, Snap, Flatpak, user-namespace
        // restrictions). Fall through to alternatives if any opener fails.
        let openers = [
            "xdg-open",
            "sensible-browser",
            "x-www-browser",
            "firefox",
            "google-chrome",
            "chromium",
            "chromium-browser",
        ];
        for opener in &openers {
            let result = std::process::Command::new(opener)
                .arg(url)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
            if result.is_ok() {
                return true;
            }
        }
        false
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        let _ = url;
        false
    }
}

pub fn create_basic_dir() -> AppResult {
    create_dir_all(crate::DOWN_PATH)?;
    create_dir_all(crate::UNZIP_PATH)?;
    create_dir_all(crate::BACKUP_PATH)?;
    create_dir_all(crate::FILEDB_PATH)?;
    create_dir_all(crate::ZIGBEE_IMG_PATH)?;
    create_dir_all(crate::MQTT_CERTS_PATH)?;
    create_dir_all(crate::RESOURCE_PATH)?;
    Ok(())
}
