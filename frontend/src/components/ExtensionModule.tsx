import React, { useState, useEffect, useRef, Suspense } from 'react';
import { RefreshCw, AlertCircle } from 'lucide-react';

interface ExtensionModuleProps {
    src: string;
    extensionId: string;
    props?: Record<string, unknown>;
}

interface ExtensionContext {
    extensionId: string;
    theme: string;
    api: {
        navigate: (path: string) => void;
        showToast: (message: string, type?: 'success' | 'error' | 'info') => void;
        getToken: () => string | null;
    };
    [key: string]: unknown;
}

type ExtensionComponent = React.ComponentType<{ context: ExtensionContext }>;

/**
 * ES Module loader for UI extensions
 * 
 * This component dynamically imports ES modules from extension URLs.
 * 
 * SECURITY WARNING: ES module extensions run with full page access.
 * Only use this for trusted extensions with verified signatures.
 * 
 * Extensions should export a default React component:
 * ```
 * export default function MyExtension({ context }) {
 *   return <div>Extension Content</div>;
 * }
 * ```
 */
export function ExtensionModule({ src, extensionId, props = {} }: ExtensionModuleProps) {
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [Component, setComponent] = useState<ExtensionComponent | null>(null);
    const mountedRef = useRef(true);

    useEffect(() => {
        mountedRef.current = true;
        return () => {
            mountedRef.current = false;
        };
    }, []);

    useEffect(() => {
        setLoading(true);
        setError(null);
        setComponent(null);

        const loadModule = async () => {
            try {
                // Dynamically import the ES module
                const module = await import(/* @vite-ignore */ src);
                
                if (!mountedRef.current) return;

                // Check for default export (React component)
                if (module.default && typeof module.default === 'function') {
                    setComponent(() => module.default);
                } else if (module.Extension && typeof module.Extension === 'function') {
                    // Alternative named export
                    setComponent(() => module.Extension);
                } else {
                    throw new Error('Extension module must export a default React component');
                }

                setLoading(false);
            } catch (e) {
                if (!mountedRef.current) return;
                console.error('Failed to load extension module:', e);
                setError(e instanceof Error ? e.message : 'Failed to load extension');
                setLoading(false);
            }
        };

        loadModule();
    }, [src]);

    // Context object passed to extension
    const context: ExtensionContext = {
        extensionId,
        theme: document.documentElement.classList.contains('dark') ? 'dark' : 'light',
        // Add API methods for extensions to use
        api: {
            navigate: (path: string) => {
                if (path.startsWith('/') && !path.startsWith('//')) {
                    window.location.href = path;
                }
            },
            showToast: (message: string, type: 'success' | 'error' | 'info' = 'info') => {
                // TODO: Integrate with toast system
                console.log(`[Extension Toast] ${type}: ${message}`);
            },
            getToken: () => {
                // Return auth token for API calls
                return localStorage.getItem('token');
            },
        },
        ...props,
    };

    if (loading) {
        return (
            <div className="flex items-center justify-center h-full bg-gray-50 dark:bg-gray-900">
                <div className="text-center">
                    <RefreshCw className="h-8 w-8 text-gray-400 animate-spin mx-auto" />
                    <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
                        Loading extension module...
                    </p>
                </div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="flex items-center justify-center h-full bg-gray-50 dark:bg-gray-900">
                <div className="text-center max-w-md px-4">
                    <AlertCircle className="h-12 w-12 text-red-400 mx-auto" />
                    <h3 className="mt-4 text-lg font-medium text-gray-900 dark:text-white">
                        Failed to load extension
                    </h3>
                    <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
                        {error}
                    </p>
                    <button
                        onClick={() => {
                            setLoading(true);
                            setError(null);
                            // Re-trigger the effect
                            setComponent(null);
                        }}
                        className="mt-4 px-4 py-2 text-sm font-medium text-primary-600 hover:text-primary-700"
                    >
                        Retry
                    </button>
                </div>
            </div>
        );
    }

    if (!Component) {
        return null;
    }

    return (
        <div className="h-full overflow-auto">
            <ErrorBoundary extensionId={extensionId}>
                <Suspense fallback={
                    <div className="flex items-center justify-center h-full">
                        <RefreshCw className="h-6 w-6 text-gray-400 animate-spin" />
                    </div>
                }>
                    <Component context={context} />
                </Suspense>
            </ErrorBoundary>
        </div>
    );
}

// Error boundary for extension components
interface ErrorBoundaryProps {
    extensionId: string;
    children: React.ReactNode;
}

interface ErrorBoundaryState {
    hasError: boolean;
    error: Error | null;
}

class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
    constructor(props: ErrorBoundaryProps) {
        super(props);
        this.state = { hasError: false, error: null };
    }

    static getDerivedStateFromError(error: Error) {
        return { hasError: true, error };
    }

    componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
        console.error(`Extension ${this.props.extensionId} error:`, error, errorInfo);
    }

    render() {
        if (this.state.hasError) {
            return (
                <div className="flex items-center justify-center h-full bg-gray-50 dark:bg-gray-900">
                    <div className="text-center max-w-md px-4">
                        <AlertCircle className="h-12 w-12 text-red-400 mx-auto" />
                        <h3 className="mt-4 text-lg font-medium text-gray-900 dark:text-white">
                            Extension Error
                        </h3>
                        <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
                            The extension encountered an error: {this.state.error?.message}
                        </p>
                        <button
                            onClick={() => this.setState({ hasError: false, error: null })}
                            className="mt-4 px-4 py-2 text-sm font-medium text-primary-600 hover:text-primary-700"
                        >
                            Retry
                        </button>
                    </div>
                </div>
            );
        }

        return this.props.children;
    }
}

