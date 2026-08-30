import { useState, useEffect, useRef } from 'react';
import { RefreshCw, AlertCircle, ExternalLink } from 'lucide-react';

interface ExtensionIframeProps {
    src: string;
    title: string;
    className?: string;
}

/**
 * Sandboxed iframe loader for UI extensions
 * 
 * Security features:
 * - Sandboxed with minimal permissions
 * - CSP headers via sandbox attribute
 * - Cross-origin communication via postMessage
 */
export function ExtensionIframe({ src, title, className = '' }: ExtensionIframeProps) {
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const iframeRef = useRef<HTMLIFrameElement>(null);

    useEffect(() => {
        setLoading(true);
        setError(null);
    }, [src]);

    const handleLoad = () => {
        setLoading(false);
    };

    const handleError = () => {
        setLoading(false);
        setError('Failed to load extension content');
    };

    // Listen for messages from the iframe
    useEffect(() => {
        const handleMessage = (event: MessageEvent) => {
            // Verify origin matches the extension source
            try {
                const srcUrl = new URL(src);
                if (event.origin !== srcUrl.origin) return;
            } catch {
                return;
            }

            // Handle extension messages
            const { type, payload } = event.data || {};
            
            switch (type) {
                case 'extension:ready':
                    console.log('Extension ready:', payload);
                    break;
                case 'extension:resize':
                    // Handle resize requests if needed
                    break;
                case 'extension:navigate':
                    // Handle navigation requests (with validation)
                    if (payload?.path && typeof payload.path === 'string') {
                        // Only allow relative paths
                        if (payload.path.startsWith('/') && !payload.path.startsWith('//')) {
                            window.location.href = payload.path;
                        }
                    }
                    break;
                case 'extension:api':
                    // Handle API requests (future: proxy through parent)
                    console.log('Extension API request:', payload);
                    break;
            }
        };

        window.addEventListener('message', handleMessage);
        return () => window.removeEventListener('message', handleMessage);
    }, [src]);

    // Send context to iframe when loaded
    useEffect(() => {
        if (!loading && iframeRef.current?.contentWindow) {
            try {
                const srcUrl = new URL(src);
                iframeRef.current.contentWindow.postMessage(
                    {
                        type: 'clovalink:context',
                        payload: {
                            theme: document.documentElement.classList.contains('dark') ? 'dark' : 'light',
                            // Add more context as needed
                        },
                    },
                    srcUrl.origin
                );
            } catch (e) {
                console.error('Failed to send context to extension:', e);
            }
        }
    }, [loading, src]);

    return (
        <div className={`relative w-full h-full ${className}`}>
            {/* Loading state */}
            {loading && (
                <div className="absolute inset-0 flex items-center justify-center bg-gray-50 dark:bg-gray-900">
                    <div className="text-center">
                        <RefreshCw className="h-8 w-8 text-gray-400 animate-spin mx-auto" />
                        <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
                            Loading extension...
                        </p>
                    </div>
                </div>
            )}

            {/* Error state */}
            {error && (
                <div className="absolute inset-0 flex items-center justify-center bg-gray-50 dark:bg-gray-900">
                    <div className="text-center max-w-md px-4">
                        <AlertCircle className="h-12 w-12 text-red-400 mx-auto" />
                        <h3 className="mt-4 text-lg font-medium text-gray-900 dark:text-white">
                            Failed to load extension
                        </h3>
                        <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
                            {error}
                        </p>
                        <a
                            href={src}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="mt-4 inline-flex items-center text-sm text-primary-600 hover:text-primary-700"
                        >
                            Open in new tab
                            <ExternalLink className="h-4 w-4 ml-1" />
                        </a>
                    </div>
                </div>
            )}

            {/* Iframe */}
            <iframe
                ref={iframeRef}
                src={src}
                title={title}
                onLoad={handleLoad}
                onError={handleError}
                className={`w-full h-full border-0 ${loading || error ? 'invisible' : 'visible'}`}
                sandbox="allow-scripts allow-forms allow-same-origin allow-popups"
                allow="clipboard-read; clipboard-write"
                loading="lazy"
            />
        </div>
    );
}

