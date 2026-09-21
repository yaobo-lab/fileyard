use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub struct MqttClientConnectedEvent {
    pub username: String,
    pub client_id: String,
    pub ip_address: Option<String>,
    pub port: Option<i32>,
    pub proto_ver: Option<i16>,
    pub keepalive: Option<i32>,
    pub clean_start: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct MqttClientDisconnectedEvent {
    pub username: Option<String>,
    pub client_id: Option<String>,
    pub reason: Option<String>,
}

pub type ClientConnectedHook = Box<dyn Fn(MqttClientConnectedEvent) + Send + Sync + 'static>;
pub type ClientDisconnectedHook = Box<dyn Fn(MqttClientDisconnectedEvent) + Send + Sync + 'static>;

static CONNECTED_HOOK: OnceLock<ClientConnectedHook> = OnceLock::new();
static DISCONNECTED_HOOK: OnceLock<ClientDisconnectedHook> = OnceLock::new();

/// 设置客户端连接成功回调
pub fn set_on_client_connected<F>(f: F)
where
    F: Fn(MqttClientConnectedEvent) + Send + Sync + 'static,
{
    let _ = CONNECTED_HOOK.set(Box::new(f));
}

/// 设置客户端断开连接回调
pub fn set_on_client_disconnected<F>(f: F)
where
    F: Fn(MqttClientDisconnectedEvent) + Send + Sync + 'static,
{
    let _ = DISCONNECTED_HOOK.set(Box::new(f));
}

/// 触发客户端连接事件
pub fn notify_client_connected(event: MqttClientConnectedEvent) {
    if let Some(hook) = CONNECTED_HOOK.get() {
        hook(event);
    }
}

/// 触发客户端断开连接事件
pub fn notify_client_disconnected(event: MqttClientDisconnectedEvent) {
    if let Some(hook) = DISCONNECTED_HOOK.get() {
        hook(event);
    }
}
