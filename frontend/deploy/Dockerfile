# ─── Stage 1: Build Stage ──────────────────────────────────────────────────
FROM crpi-p35inr9lrgirg3t0.cn-hangzhou.personal.cr.aliyuncs.com/yaobo-box/bun-alpine:1.4 AS builder

WORKDIR /app

# 优先复制依赖声明文件以利用 Docker 缓存层
COPY package.json bun.lock ./
RUN bun install --frozen-lockfile

# 复制前端项目所有源码
COPY . .

# 执行生产打包，生成 dist 目录
RUN bun run build

# ─── Stage 2: Runtime Stage ────────────────────────────────────────────────
FROM crpi-p35inr9lrgirg3t0.cn-hangzhou.personal.cr.aliyuncs.com/yaobo-box/nginx:1.15.6.1-alpine  AS runner

# 清空默认静态文件
RUN rm -rf /usr/share/nginx/html/*

# 复制打包好的前端静态资源
COPY --from=builder /app/dist /usr/share/nginx/html

# 复制生产环境 Nginx 配置文件
COPY deploy/nginx.conf /etc/nginx/nginx.conf

# 复制默认生产环境运行配置（外部部署可通过 -v 挂载自定义 config.js 覆盖）
COPY deploy/config.production.js /usr/share/nginx/html/config.js

EXPOSE 8005

STOPSIGNAL SIGQUIT

CMD ["nginx", "-g", "daemon off;"]
