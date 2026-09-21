use rmqtt::{
    args::CommandArgs,
    context::ServerContext,
    net::{Builder, tls_provider},
    node::Node,
    server::MqttServer,
};
use rmqtt_conf::{Options, Settings, listener::Listener};

mod plugin;

pub fn config_builder(cfg: &Listener) -> Builder {
    Builder::new()
        .name(cfg.name.as_str())
        .laddr(cfg.addr)
        .max_connections(cfg.max_connections)
        .max_handshaking_limit(cfg.max_handshaking_limit)
        .max_packet_size(cfg.max_packet_size.as_u32())
        .backlog(cfg.backlog)
        .nodelay(cfg.nodelay)
        .reuseaddr(cfg.reuseaddr)
        .reuseport(cfg.reuseport)
        .allow_anonymous(cfg.allow_anonymous)
        .min_keepalive(cfg.min_keepalive)
        .max_keepalive(cfg.max_keepalive)
        .allow_zero_keepalive(cfg.allow_zero_keepalive)
        .keepalive_backoff(cfg.keepalive_backoff)
        .max_inflight(cfg.max_inflight)
        .handshake_timeout(cfg.handshake_timeout)
        .max_mqueue_len(cfg.max_mqueue_len)
        .mqueue_rate_limit(cfg.mqueue_rate_limit.0, cfg.mqueue_rate_limit.1)
        .max_clientid_len(cfg.max_clientid_len)
        .max_qos_allowed(cfg.max_qos_allowed)
        .max_topic_levels(cfg.max_topic_levels)
        .session_expiry_interval(cfg.session_expiry_interval)
        .max_session_expiry_interval(cfg.max_session_expiry_interval)
        .message_retry_interval(cfg.message_retry_interval)
        .message_expiry_interval(cfg.message_expiry_interval)
        .max_subscriptions(cfg.max_subscriptions)
        .max_topic_aliases(cfg.max_topic_aliases)
        .tls_cross_certificate(cfg.cross_certificate)
        .tls_cert(cfg.cert.clone())
        .tls_key(cfg.key.clone())
        .limit_subscription(cfg.limit_subscription)
        .delayed_publish(cfg.delayed_publish)
        .proxy_protocol(cfg.proxy_protocol)
        .proxy_protocol_timeout(cfg.proxy_protocol_timeout)
}

pub fn config_args(cfg: &Settings) -> CommandArgs {
    CommandArgs {
        node_id: cfg.opts.node_id,
        plugins_default_startups: cfg.opts.plugins_default_startups.clone(),
        node_grpc_addrs: cfg.opts.node_grpc_addrs.clone(),
        raft_peer_addrs: cfg.opts.raft_peer_addrs.clone(),
        raft_leader_id: cfg.opts.raft_leader_id,
    }
}

/// 启动并运行 MQTT 服务器
pub async fn run_server(config_path: &str, plugins_dir: Option<&str>) -> anyhow::Result<()> {
    log::info!("正在启动 MQTT 服务器，配置文件: {}", config_path);

    let opts = Options {
        cfg_name: Some(config_path.into()),
        ..Default::default()
    };
    let conf = Settings::init(opts).map_err(|e| anyhow::anyhow!("MQTT配置初始化失败: {e}"))?;

    // 注册全局默认 crypto provider（若已安装则忽略）
    let _ = tls_provider::default_provider().install_default();

    let _ = Settings::logs();

    // 节点信息
    let node = Node::new(
        conf.node.id,
        conf.node.busy.loadavg,
        conf.node.busy.cpuloadavg,
        conf.node.busy.update_interval,
    );

    let p_dir = plugins_dir.unwrap_or(conf.plugins.dir.as_str());

    // 初始化 ServerContext
    let scx = ServerContext::new()
        .args(config_args(conf))
        .node(node)
        .task_exec_workers(conf.task.exec_workers)
        .task_exec_queue_max(conf.task.exec_queue_max)
        .busy_check_enable(conf.node.busy.check_enable)
        .busy_handshaking_limit(conf.node.busy.handshaking)
        .mqtt_delayed_publish_max(conf.mqtt.delayed_publish_max)
        .mqtt_delayed_publish_immediate(conf.mqtt.delayed_publish_immediate)
        .mqtt_max_sessions(conf.mqtt.max_sessions)
        .plugins_config_dir(p_dir)
        .build()
        .await;

    // 注册插件
    plugin::registers(&scx, conf.plugins.default_startups.clone())
        .await
        .map_err(|e| anyhow::anyhow!("MQTT插件注册失败: {e}"))?;

    let mut mqtt_svr = MqttServer::new(scx);

    // 注册 TCP 监听器
    for (_, listen_cfg) in Settings::instance().listeners.tcps.iter() {
        mqtt_svr = mqtt_svr.listener(
            config_builder(listen_cfg)
                .bind()
                .map_err(|e| anyhow::anyhow!("TCP bind失败: {e}"))?
                .tcp()
                .map_err(|e| anyhow::anyhow!("TCP listener启动失败: {e}"))?,
        );
    }

    // 注册 TLS 监听器
    for (_, listen_cfg) in Settings::instance().listeners.tlss.iter() {
        mqtt_svr = mqtt_svr.listener(
            config_builder(listen_cfg)
                .bind()
                .map_err(|e| anyhow::anyhow!("TLS bind失败: {e}"))?
                .tls()
                .map_err(|e| anyhow::anyhow!("TLS listener启动失败: {e}"))?,
        );
    }

    log::info!("MQTT 服务器已成功初始化并开始监听！");
    mqtt_svr
        .build()
        .run()
        .await
        .map_err(|e| anyhow::anyhow!("MQTT 服务器运行错误: {e}"))?;

    Ok(())
}
