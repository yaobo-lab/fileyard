/**
 * Fileyard 前端生产环境运行时配置
 * 可以在容器启动时通过挂载替换该文件：
 * docker run -v /path/to/config.production.js:/usr/share/nginx/html/config.js ...
 */
window.__APP_CONFIG__ = {
    // 应用名称
    APP_NAME: "Fileyard-Frontend",

    // 后端 API 接口地址
    // 留空字符串 '' 代表使用当前站点的同源相对路径（由 Nginx 代理 /api 到后端）
    // 若使用独立的 API 域名或负载均衡器，可在此配置，例如：'https://api.yourdomain.com'
    API_URL: "http://127.0.0.1:3000",

    // 是否开启前端调试模式
    DEBUG: false,
};
