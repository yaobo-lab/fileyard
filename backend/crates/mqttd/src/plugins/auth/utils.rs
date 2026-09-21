#![allow(dead_code)]
use aes::Aes128;
use base64::{Engine as _, engine::general_purpose};
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use rand::Rng;
type Aes128Cbc = Cbc<Aes128, Pkcs7>;

pub fn aes128_cbc_encrypt(
    plaintext: &str,
    key: &str,
    iv: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // 检查密钥和IV长度
    if key.len() != 16 {
        return Err("Key must be 16 bytes (128 bits) for AES-256".into());
    }
    if iv.len() != 16 {
        return Err("IV must be 16 bytes".into());
    }

    // 准备密钥和IV
    let key_bytes = key.as_bytes();
    let iv_bytes = iv.as_bytes();

    // 创建加密器
    let cipher = Aes128Cbc::new_from_slices(key_bytes, iv_bytes)?;

    // 加密数据
    let ciphertext = cipher.encrypt_vec(plaintext.as_bytes());

    // 返回Base64编码结果
    Ok(general_purpose::STANDARD.encode(ciphertext))
}

pub fn aes128_cbc_decrypt(
    ciphertext: &str,
    key: &str,
    iv: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    // 检查密钥和IV长度
    if key.len() != 16 {
        return Err("Key must be 16 bytes (128 bits) for AES-256".into());
    }
    if iv.len() != 16 {
        return Err("IV must be 16 bytes".into());
    }

    // 准备密钥和IV
    let key_bytes = key.as_bytes();
    let iv_bytes = iv.as_bytes();

    // 创建解密器
    let cipher = Aes128Cbc::new_from_slices(key_bytes, iv_bytes)?;

    // 解码Base64
    let ciphertext_bytes = general_purpose::STANDARD.decode(ciphertext)?;

    // 解密数据
    let plaintext_bytes = cipher.decrypt_vec(&ciphertext_bytes)?;

    // 转换为字符串
    Ok(String::from_utf8(plaintext_bytes)?)
}

//获取网关注册密钥：规则：ip_mid_productkey_rand3)==key
pub fn crate_passwd(
    ipaddr: &str,
    mid: &str,
    sku_id: &str,
    secret_key: &str,
    iv_val: &str,
) -> String {
    let mut rng = rand::rng();
    let random_string: String = (0..3)
        .map(|_| (b'A' + rng.random_range(0..26)) as char)
        .collect();
    let txt = format!("{}_{}_{}_{}", ipaddr, mid, sku_id, random_string);
    let encrypted = aes128_cbc_encrypt(&txt, secret_key, iv_val).unwrap();
    encrypted
}

pub fn to_topic(topic: &str, skuid: &str, uuid: &str) -> String {
    topic.replace("{skuid}", skuid).replace("{uuid}", uuid)
}

// /aam/sub/request/2928/10002 是否符合 /aam/sub/request/+/+
pub fn topic_match_one(topic: &str, pattern_topic: &str) -> bool {
    if topic.to_lowercase() == pattern_topic.to_lowercase() {
        return true;
    }

    let topic_parts: Vec<&str> = topic.split('/').collect();
    let pattern_parts: Vec<&str> = pattern_topic.split('/').collect();
    if topic_parts.len() != pattern_parts.len() {
        return false;
    }

    for (i, pattern_part) in pattern_parts.iter().enumerate() {
        if *pattern_part != "+" && *pattern_part != topic_parts[i] {
            return false;
        }
    }
    true
}

// /aam/sub/request/2928/10002 是否符合 /aam/sub/request/+/+
// 提取 + + 里的值
pub fn topic_get_match_one(topic: &str, pattern_topic: &str) -> Option<Vec<String>> {
    let topic_parts: Vec<&str> = topic.split('/').collect();
    let pattern_parts: Vec<&str> = pattern_topic.split('/').collect();
    if topic_parts.len() != pattern_parts.len() {
        return None;
    }

    let mut values = Vec::new();
    for (i, pattern_part) in pattern_parts.iter().enumerate() {
        if *pattern_part == "+" {
            values.push(topic_parts[i].to_string());
        } else if *pattern_part != topic_parts[i] {
            return None; // 静态部分不匹配
        }
    }
    Some(values)
}

//test/topic/1/21/2232  能配符配置 test/topic/#
pub fn topic_match_all(topic: &str, pattern_topic: &str) -> bool {
    if topic.to_lowercase() == pattern_topic.to_lowercase() {
        return true;
    }

    let topic_parts: Vec<&str> = topic.split('/').collect();
    let filter_parts: Vec<&str> = pattern_topic.split('/').collect();

    // 检查是否存在 # 通配符
    if let Some(pos) = filter_parts.iter().position(|&x| x == "#") {
        // 检查静态前缀是否匹配
        if topic_parts.len() < pos {
            return false;
        }

        for i in 0..pos {
            if filter_parts[i] != "+" && filter_parts[i] != topic_parts[i] {
                return false;
            }
        }

        true
    } else {
        // 没有 # 通配符时需要完全匹配
        if topic_parts.len() != filter_parts.len() {
            return false;
        }

        for (i, &filter_part) in filter_parts.iter().enumerate() {
            if filter_part != "+" && filter_part != topic_parts[i] {
                return false;
            }
        }

        true
    }
}

/*
密文：RdrD10UcUipdSNQvscnBrw==
原文：424195

AesSecretKey：Yas9MIzk1Cj1cSGL
AesIv：3605146757424653
*/
#[cfg(test)]
#[allow(dead_code)]
#[allow(unused_imports)]
mod tests {

    use super::*;
    use toolkit_rs::logger::{self, LogConfig};

    pub const secret_key: &str = "Yas9MIzk1Cj1cSGL";
    pub const iv_val: &str = "3605146757424653";
    fn init_log() {
        logger::setup(LogConfig::default()).expect("Logger init failed.");
    }

    #[test]
    fn test_match_all() {
        init_log();
        println!(
            "match all: {}",
            topic_match_all(
                "/aam/shop/sub/msg/AAM2668/C498940000C22D87",
                "/aam/shop/sub/msg/#"
            )
        );

        println!(
            "match one: {}",
            topic_match_one(
                "/aam/shop/sub/msg/AAM2668/C498940000C22D87",
                "/aam/shop/sub/msg/+/+"
            )
        );
    }

    #[test]
    fn test_encrypt() {
        init_log();

        // 配置参数
        // user_name skuid9383
        // uuid2838212 clientid
        // 加密结果(密码): ku4S60HdtInAyAlbGCSc0IE16QgJSMvvPVy9KFtVwSSrbwPGvXzV6HTe4+9hQOm8
        //  ku4S60HdtInAyAlbGCSc0IE16QgJSMvvPVy9KFtVwSSrbwPGv
        let plaintext = "10.0.3.36|uuid2838212|skuid9383|123456";

        // 加密
        let encrypted = aes128_cbc_encrypt(plaintext, secret_key, iv_val).unwrap();
        println!("加密结果: {}", encrypted);

        // 解密验证
        let decrypted = aes128_cbc_decrypt(&encrypted, secret_key, iv_val).unwrap();
        println!("解密结果: {}", decrypted);

        let res: Vec<&str> = decrypted.split("|").collect();
        if res.len() != 4 {
            panic!("配置参数格式错误");
        }

        for i in res {
            println!("res-->{}", i);
        }
    }

    #[test]
    fn test_dev() {
        init_log();

        // 配置参数
        let plaintext = "10.0.3.36|client-id-rust-0001|aam_sub_rust|123456";

        // 加密
        let encrypted = aes128_cbc_encrypt(plaintext, secret_key, iv_val).unwrap();
        println!("加密结果: {}", encrypted);

        // 解密验证
        let decrypted = aes128_cbc_decrypt(&encrypted, secret_key, iv_val).unwrap();
        println!("解密结果: {}", decrypted);

        let res: Vec<&str> = decrypted.split("|").collect();
        if res.len() != 4 {
            panic!("配置参数格式错误");
        }

        for i in res {
            println!("res-->{}", i);
        }
    }

    #[test]
    fn test_create_ha_admin_account() {
        init_log();

        // 配置参数
        // usr_name=aam_admin_homeassistant
        let plaintext = "10.0.3.253|homeassistant-0001|ha_gw|12345";

        // 加密
        let encrypted = aes128_cbc_encrypt(plaintext, secret_key, iv_val).unwrap();
        println!("加密结果: {}", encrypted);

        // 解密验证
        let decrypted = aes128_cbc_decrypt(&encrypted, secret_key, iv_val).unwrap();
        println!("解密结果: {}", decrypted);

        let res: Vec<&str> = decrypted.split("|").collect();
        if res.len() != 4 {
            panic!("配置参数格式错误");
        }

        for i in res {
            println!("res-->{}", i);
        }
    }

    #[test]
    fn test_encrypt_sub() {
        init_log();

        // 配置参数
        let plaintext = "10.0.3.85|openserver|openserver|hfkIq";

        // 加密
        let encrypted = aes128_cbc_encrypt(plaintext, secret_key, iv_val).unwrap();
        println!("加密结果: {}", encrypted);

        // 解密验证
        let decrypted = aes128_cbc_decrypt(&encrypted, secret_key, iv_val).unwrap();
        println!("解密结果: {}", decrypted);

        let res: Vec<&str> = decrypted.split("|").collect();
        if res.len() != 4 {
            panic!("配置参数格式错误");
        }

        for i in res {
            println!("res-->{}", i);
        }
    }

    #[test]
    fn test_encrypt_admin() {
        init_log();

        // 配置参数
        // usr_name=aam_admin_mqttx
        let plaintext = "10.0.3.36|mqttx-client-001|mqttx_suk|12345";

        // 加密
        let encrypted = aes128_cbc_encrypt(plaintext, secret_key, iv_val).unwrap();
        println!("加密结果: {}", encrypted);

        // 解密验证
        let decrypted = aes128_cbc_decrypt(&encrypted, secret_key, iv_val).unwrap();
        println!("解密结果: {}", decrypted);

        let res: Vec<&str> = decrypted.split("|").collect();
        if res.len() != 4 {
            panic!("配置参数格式错误");
        }

        for i in res {
            println!("res-->{}", i);
        }
    }

    #[test]
    fn test_decode() {
        init_log();

        let encrypted = "UrwdW7d/A/R28PqvDIVQ/Y+n6M2ULvbnIn0XvGWkdQxBwQ+544surPy/lUakt52GgKyX2Ip3Ij66wFf2mbBC4Q==";
        let decrypted = aes128_cbc_decrypt(&encrypted, secret_key, iv_val).unwrap();
        println!("解密结果: {}", decrypted);

        let res: Vec<&str> = decrypted.split("|").collect();
        if res.len() != 4 {
            panic!("配置参数格式错误");
        }

        for i in res {
            println!("res-->{}", i);
        }
    }
}
