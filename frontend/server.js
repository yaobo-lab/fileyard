import express from 'express';
import { createProxyMiddleware } from 'http-proxy-middleware';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const dev = process.env.NODE_ENV !== 'production';
const port = process.env.PORT || 8080;

const startServer = async () => {
    const app = express();

    // 1. Proxy API requests to the Rust backend service
    const backendUrl = process.env.BACKEND_URL || 'http://192.168.3.42:8001';
    app.use('/api', createProxyMiddleware({
        target: backendUrl,
        changeOrigin: true,
        secure: false,
    }));

    // 2. Route routing for Next.js /storage, /_next, and /__nextjs_font requests
    if (dev) {
        // Dev: proxy everything to Next.js dev server on port 3000 with complete header rewrite
        // This completely bypasses Turbopack CORS/Cross-Origin Referer 403 Forbidden checks
        app.use(createProxyMiddleware({
            target: 'http://localhost:3000',
            changeOrigin: true,
            ws: true,
            filter: (pathname) => (
                pathname.startsWith('/storage') ||
                pathname.startsWith('/_next') ||
                pathname.startsWith('/__nextjs_font')
            ),
            on: {
                proxyReq: (proxyReq, req) => {
                    proxyReq.setHeader('host', 'localhost:3000');
                    proxyReq.setHeader('origin', 'http://localhost:3000');
                    if (req.headers.referer) {
                        proxyReq.setHeader('referer', req.headers.referer.replace(/^https?:\/\/[^/]+/, 'http://localhost:3000'));
                    }
                }
            }
        }));
    } else {
        // Prod: lazy import next and load custom handler in production
        const { default: next } = await import('next');
        const nextApp = next({ dev, dir: path.resolve(__dirname, './storageui') });
        const nextHandler = nextApp.getRequestHandler();
        await nextApp.prepare();
        
        app.all(/^\/storage($|\/.*)/, (req, res) => nextHandler(req, res));
        app.all(/^\/_next($|\/.*)/, (req, res) => nextHandler(req, res));
    }

    // 3. Serve frontend SPA pages and assets
    if (dev) {
        // Dev: proxy everything else to Vite dev server on port 8081 for HMR
        app.use('/', createProxyMiddleware({
            target: 'http://localhost:8081',
            changeOrigin: true,
            ws: true,
        }));
    } else {
        // Prod: serve built static files from dist directory
        const distPath = path.resolve(__dirname, './dist');
        app.use(express.static(distPath));
        app.get('*', (req, res) => {
            res.sendFile(path.join(distPath, 'index.html'));
        });
    }

    app.listen(port, () => {
        console.log(`> Unified Server ready on http://localhost:${port}`);
    });
};

startServer().catch((err) => {
    console.error('Error starting server:', err);
    process.exit(1);
});
