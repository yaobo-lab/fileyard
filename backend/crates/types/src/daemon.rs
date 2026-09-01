use anyhow::anyhow;
use std::path::Path;
use toolkit_rs::AppResult;

/// Daemon info written to `~/.openfang/daemon.json` so the CLI can find us.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct DaemonInfo {
    pub pid: u32,
    pub listen_addr: String,
    pub started_at: String,
    pub version: String,
    pub platform: String,
}

pub fn wait_shutdown() -> tokio::sync::oneshot::Receiver<()> {
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

    // Listen for Ctrl+C
    tokio::spawn(async move {
        #[cfg(target_os = "windows")]
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for ctrl-c");
        #[cfg(not(target_os = "windows"))]
        {
            use tokio::signal::unix::{signal, SignalKind};
            let mut term_signal =
                signal(SignalKind::terminate()).expect("Failed to create signal handler");
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = term_signal.recv() => {}
            }
        }
        shutdown_tx
            .send(())
            .expect("Failed to send the shutdown command");
    });

    shutdown_rx
}

/// Get the OpenFang home directory, respecting OPENFANG_HOME env var.
pub fn home_dir() -> std::path::PathBuf {
    std::env::current_dir().unwrap()
}

pub fn daemon_path() -> std::path::PathBuf {
    let p = format!("{}{}", crate::FILEDB_PATH, "daemon.json");
    std::path::PathBuf::from(p)
}
//检查守护进程
pub async fn health() -> Option<String> {
    let info = read_daemon_info()?;

    let http_host_uri = info.listen_addr.replace("0.0.0.0", "127.0.0.1");
    let health_uri = format!("{}/api/health", http_host_uri);

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(1))
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .ok()?;

    let resp = client.get(&health_uri).send().await.ok()?;
    if resp.status().is_success() {
        Some(http_host_uri)
    } else {
        None
    }
}

pub fn read_daemon_info() -> Option<DaemonInfo> {
    let info_path = daemon_path();
    let contents = std::fs::read_to_string(info_path).ok()?;
    serde_json::from_str(&contents).ok()
}

pub fn set_daemon_info(addr: String, version: String) -> AppResult {
    let info_path = &daemon_path();
    if info_path.exists() {
        if let Ok(existing) = std::fs::read_to_string(info_path) {
            if let Ok(info) = serde_json::from_str::<DaemonInfo>(&existing) {
                if is_process_alive(info.pid) && is_daemon_responding(&info.listen_addr) {
                    return Err(anyhow!(
                        "Another daemon (PID {}) is already running at {}",
                        info.pid,
                        info.listen_addr
                    ));
                }
            }
        }
        let _ = std::fs::remove_file(info_path);
    }

    let daemon_info = DaemonInfo {
        pid: std::process::id(),
        listen_addr: addr,
        started_at: chrono::Utc::now().to_rfc3339(),
        version,
        platform: std::env::consts::OS.to_string(),
    };

    if let Ok(json) = serde_json::to_string_pretty(&daemon_info) {
        let _ = std::fs::write(info_path, json);
        restrict_permissions(info_path);
    }

    Ok(())
}

pub async fn start_daemon_background() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("Cannot find executable: {e}"))?;

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        std::process::Command::new(&exe)
            .arg("start")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
            .spawn()
            .map_err(|e| format!("Failed to spawn daemon: {e}"))?;
    }

    #[cfg(not(windows))]
    {
        std::process::Command::new(&exe)
            .arg("start")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn daemon: {e}"))?;
    }

    // Poll for daemon readiness
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if health().await.is_some() {
            return Ok(());
        }
    }

    Err("Daemon did not become ready within 10 seconds".to_string())
}
pub fn daemon_client() -> reqwest::Client {
    let builder = reqwest::Client::builder().timeout(std::time::Duration::from_secs(120));
    builder.build().expect("Failed to build HTTP client")
}

/// Check if a process with the given PID is still alive.
pub fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // Use kill -0 to check if process exists without sending a signal
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(windows)]
    {
        // tasklist /FI "PID eq N" returns "INFO: No tasks..." when no match,
        // or a table row with the PID when found. Check exit code and that
        // "INFO:" is NOT in the output to confirm the process exists.
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .map(|o| {
                o.status.success() && {
                    let out = String::from_utf8_lossy(&o.stdout);
                    !out.contains("INFO:") && out.contains(&pid.to_string())
                }
            })
            .unwrap_or(false)
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false
    }
}

pub fn is_daemon_responding(addr: &str) -> bool {
    let addr_only = addr
        .strip_prefix("http://")
        .or_else(|| addr.strip_prefix("https://"))
        .unwrap_or(addr);
    if let Ok(sock_addr) = addr_only.parse::<std::net::SocketAddr>() {
        std::net::TcpStream::connect_timeout(&sock_addr, std::time::Duration::from_millis(500))
            .is_ok()
    } else {
        // Fallback: try connecting to hostname
        std::net::TcpStream::connect(addr_only)
            .map(|_| true)
            .unwrap_or(false)
    }
}

#[cfg(unix)]
pub fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
pub fn restrict_permissions(_path: &Path) {}

pub fn force_kill_pid(pid: u32) {
    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .output();
    }
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F"])
            .output();
    }
}
