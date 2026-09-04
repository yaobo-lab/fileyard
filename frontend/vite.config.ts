import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import path from 'path'

// https://vitejs.dev/config/
export default defineConfig({
    plugins: [
        react(),
        tailwindcss(),
    ],
    resolve: {
        alias: {
            '@': path.resolve(__dirname, './src'),
        },
    },
    server: {
        host: true,
        port: 8080,
        proxy: {
            '/api': {
                target: 'http://127.0.0.1:3000',
                changeOrigin: true,
                secure: false,
            },
            '/storage': {
                target: 'http://localhost:3000',
                changeOrigin: true,
                secure: false,
            }
        }
    },
    build: {
        rollupOptions: {
            output: {
                manualChunks: {
                    // Core React framework - rarely changes
                    vendor: ['react', 'react-dom', 'react-router-dom'],
                    // Charting library - heavy, used only on dashboard
                    charts: ['recharts'],
                    // UI utilities - used throughout
                    ui: ['lucide-react', 'clsx', 'tailwind-merge'],
                    // Date formatting - used in many places
                    dates: ['date-fns'],
                    // Grid layout - used on dashboard
                    grid: ['react-grid-layout', 'react-is'],
                }
            }
        },
        // Increase chunk size warning limit since we're intentionally splitting
        chunkSizeWarningLimit: 600,
    }
})
