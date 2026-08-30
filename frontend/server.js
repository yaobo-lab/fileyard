import express from 'express';
import next from 'next';
import { createProxyMiddleware } from 'http-proxy-middleware';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const dev = process.env.NODE_ENV !== 'production';
const port = process.env.PORT || 8080;

// Initialize Next.js app located in the subdirectory `./storageui`
const nextApp = next({ dev, dir: path.resolve(__dirname, './storageui') });
const nextHandler = nextApp.getRequestHandler();

nextApp.prepare().then(() => {
    const app = express();

    // 1. Proxy API requests to the Rust backend service
    const backendUrl = process.env.BACKEND_URL || 'http://192.168.3.42:8001';
    app.use('/api', createProxyMiddleware({
        target: backendUrl,
        changeOrigin: true,
        secure: false,
    }));

    // 2. Route routing: route /storage and /_next requests to Next.js handler
    app.use('/storage', (req, res) => nextHandler(req, res));
    app.use('/_next', (req, res) => nextHandler(req, res));

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
        console.log(`> Single Server ready on http://localhost:${port}`);
    });
}).catch((err) => {
    console.error('Error starting server:', err);
    process.exit(1);
});
