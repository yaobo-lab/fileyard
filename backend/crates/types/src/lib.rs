#![allow(
    non_upper_case_globals,
    dead_code,
    non_camel_case_types,
    non_snake_case
)]

pub mod config;

pub mod utils;

pub mod cache;
pub mod progress;
pub mod table;
pub mod ui;

pub mod daemon;

pub mod retry;

use std::{future::Future, pin::Pin};
// 异步函数返回值
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

//备份目录
pub const BACKUP_PATH: &str = "./temp/backup/";
//下载目录
pub const DOWN_PATH: &str = "./temp/download/";
//解压目录
pub const UNZIP_PATH: &str = "./temp/unzip/";
//配置文件 目录
pub const FILEDB_PATH: &str = "./etc/settings/";

//MQTT 证书 存放目录
pub const MQTT_CERTS_PATH: &str = "./etc/certs/";
//MQTT CA根证书
pub const MQTT_CERTS_DEVICE_CA: &str = "device_ca.crt";
//MQTT 客户端证书
pub const MQTT_CERTS_DEVICE_CERTIFICATE: &str = "device_cert.crt";
//MQTT 客户端私钥 key
pub const MQTT_CERTS_DEVICE_KEY: &str = "device_key.key";

//zigbee img 目录
pub const ZIGBEE_IMG_PATH: &str = "./temp/img/zigbee/";

//存放升级资源文件目录
//如果升级失败才存一个。
//Zigbee可能会继续升级，待到成功，利用影子文件补上报一个升级成功到平台
pub const RESOURCE_PATH: &str = "./temp/resource/";

pub const UPGRADE_TASK_PATH: &str = "./temp/upgrade/";
