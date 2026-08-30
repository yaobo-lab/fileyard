![storage ui cover](/public/readme/github-banner-zh-CN.png)

# Storage UI

<p align="start">
  <a href="https://storageui.dev">官网</a> ·
  <a href="https://demo.storageui.dev">演示</a> ·
  <a href="https://storageui.dev/docs">文档</a>
</p>

<p align="start">
  <a href="./README.md"><img alt="README in English" src="https://img.shields.io/badge/English-d9d9d9"></a>
  <a href="./docs/zh-CN/README.md"><img alt="简体中文文件" src="https://img.shields.io/badge/简体中文-d9d9d9"></a>
</p>

一款面向 S3、R2 和其他存储后端的开源文件浏览器，提供现代化的自托管 Web 界面，让您轻松浏览、预览、搜索和管理文件。

## 功能特性

- 图标、列表、分栏、画廊四种视图
- 搜索、筛选、排序，以及文件夹懒加载
- PDF、DOCX、XLSX、图片、文本和代码预览
- 多存储连接，数据本地持久化
- 针对环境变量配置的连接，支持按桶的只读模式
- 响应式布局与深色模式

## 快速开始

### Docker

已在 Docker Hub 发布预构建镜像。拉取后配合环境变量运行：

```bash
docker pull hahahumble/storageui

docker run -d \
  --name storage-ui \
  --restart unless-stopped \
  -p 3000:3000 \
  --env-file .env \
  hahahumble/storageui
```

运行前创建一个 `.env` 文件并填写 `STORAGE_1_*` 变量（参考 `.env.example`）。

### 使用 Vercel 部署

[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https://github.com/hahahumble/storageui)

### 本地开发

```bash
bun install
cp .env.example .env.local
bun run dev
```

打开 `http://localhost:3000`。你可以在界面中添加存储连接，或在 `.env.local` 中填写 `STORAGE_1_*` 变量来预配置服务端的桶。

## 许可

[Apache-2.0](LICENSE.md)
