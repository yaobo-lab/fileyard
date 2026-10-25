/**
 * Fileyard 前端开发/测试环境运行时配置
 * 可以在容器启动时通过挂载替换该文件：
 * docker run -v /path/to/config.dev.js:/usr/share/nginx/html/config.js ...
 */
window.__APP_CONFIG__ = {
  // 应用名称
  APP_NAME: 'Fileyard (Dev)',

  // 后端 API 接口地址
  // 若使用同源 Nginx 反向代理，留空即可（即 ''）
  // 若前后端跨域独立部署，可在此处指定后端具体地址
  API_URL: 'http://127.0.0.1:3000',

  // 是否开启前端调试模式
  DEBUG: true
};