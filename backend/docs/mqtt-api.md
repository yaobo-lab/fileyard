# MQTT 管理 API

内置 MQTT 的 REST API 通过应用 HTTP 端口提供，前缀为 `/api/mqtt`。
请求使用应用登录令牌 `Authorization: Bearer <token>`，仅 `SuperAdmin` 可访问。
MQTT 是跨租户共享实例，因此租户管理员不具有管理权限。

| 方法 | 路径 | 用途 |
| --- | --- | --- |
| GET | `/api/mqtt` | 接口目录 |
| GET | `/api/mqtt/brokers`、`/api/mqtt/nodes` | 实例信息 |
| GET | `/api/mqtt/health/check` | 健康检查 |
| GET | `/api/mqtt/clients` | 客户端搜索，保留原查询参数 |
| GET / DELETE | `/api/mqtt/clients/{clientid}` | 查询 / 踢出客户端 |
| GET | `/api/mqtt/clients/{clientid}/online` | 在线状态 |
| GET / DELETE | `/api/mqtt/clients/offlines` | 查询 / 清理离线客户端 |
| GET | `/api/mqtt/subscriptions`、`/api/mqtt/subscriptions/{clientid}` | 订阅查询 |
| GET | `/api/mqtt/routes`、`/api/mqtt/routes/{topic}` | 路由查询 |
| POST | `/api/mqtt/publish` | 发布消息 |
| POST | `/api/mqtt/subscribe`、`/api/mqtt/unsubscribe` | 管理订阅 |
| GET | `/api/mqtt/plugins` | 插件列表 |
| GET | `/api/mqtt/plugins/{node}/{plugin}` | 插件详情 |
| GET | `/api/mqtt/plugins/{node}/{plugin}/config` | 插件配置 |
| PUT | `/api/mqtt/plugins/{node}/{plugin}/config/reload` | 重载配置 |
| PUT | `/api/mqtt/plugins/{node}/{plugin}/load`、`/unload` | 启停插件 |
| GET | `/api/mqtt/stats`、`/api/mqtt/stats/sum` | 统计信息 |

发布示例：

```http
POST /api/mqtt/publish
Authorization: Bearer <应用登录令牌>
Content-Type: application/json

{"topic":"devices/test","payload":"hello","encoding":"plain","qos":0}
```

返回格式沿用原 MQTT REST API，未包装为应用业务响应。请求体最大 2 MiB。
未启用、初始化失败或已停止的 MQTT 返回 HTTP 503。

应用使用 `run_server_with_api` 在进程内共享 MQTT 上下文，不代理到其他端口。
嵌入模式不注册独立 `restapi` 插件，因此不会开启 6060 监听，也不能通过插件管理接口重新开启它。
`etc/plugins/restapi.toml` 的查询上限、消息过期时间和请求日志配置仍然生效；
独立 HTTP 监听和 Bearer Token 配置由应用路由及登录认证替代。
`plugins.default_startups` 中原有的 `restapi` 项可保留，嵌入模式会忽略它。
其他插件仍按该列表启动。

单独调用 `mqttd::run_server` 时保留原独立 REST API 行为与 `/api/v1` 路径。
