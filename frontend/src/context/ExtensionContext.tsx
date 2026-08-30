import { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import { useAuth } from './AuthContext';

// Types for extensions
export interface Extension {
    id: string;
    tenant_id: string;
    name: string;
    slug: string;
    description: string | null;
    type: 'ui' | 'file_processor' | 'automation';
    status: string;
    is_owner: boolean;
    allowed_tenant_ids: string[] | null;
    current_version: string | null;
    manifest: ExtensionManifest | null;
    created_at: string;
}

export interface InstalledExtension {
    installation_id: string;
    extension_id: string;
    name: string;
    slug: string;
    description: string | null;
    type: 'ui' | 'file_processor' | 'automation';
    version: string;
    enabled: boolean;
    settings: Record<string, unknown>;
    permissions: string[];
    installed_at: string;
}

export interface ExtensionManifest {
    name: string;
    slug: string;
    version: string;
    type: string;
    description?: string;
    entrypoint?: string;
    permissions: string[];
    webhook?: string;
    ui?: UIManifest;
    automation?: AutomationManifest;
    file_processor?: FileProcessorManifest;
}

export interface UIManifest {
    load_mode: 'iframe' | 'esm';
    sidebar: SidebarItem[];
    buttons: ButtonItem[];
    components: ComponentItem[];
}

export interface SidebarItem {
    id: string;
    extension_id: string;
    name: string;
    icon?: string;
    entrypoint: string;
    load_mode: string;
    order: number;
}

export interface ButtonItem {
    id: string;
    extension_id: string;
    name: string;
    icon?: string;
    location: string;
    entrypoint: string;
    load_mode: string;
}

export interface ComponentItem {
    id: string;
    extension_id: string;
    name: string;
    location: string;
    entrypoint: string;
    load_mode: string;
}

export interface AutomationManifest {
    default_cron?: string;
    configurable: boolean;
    config_schema?: unknown;
}

export interface FileProcessorManifest {
    file_types: string[];
    max_file_size_mb?: number;
    async_processing?: boolean;
}

export interface UIExtensionComponents {
    sidebar: SidebarItem[];
    buttons: ButtonItem[];
    components: ComponentItem[];
}

export interface AutomationJob {
    id: string;
    extension_id: string;
    tenant_id: string;
    name: string;
    cron_expression: string | null;
    next_run_at: string;
    last_run_at: string | null;
    last_status: string | null;
    last_error: string | null;
    enabled: boolean;
    config: Record<string, unknown>;
    created_at: string;
    updated_at: string;
}

interface ExtensionContextType {
    // Data
    extensions: Extension[];
    installedExtensions: InstalledExtension[];
    uiComponents: UIExtensionComponents;
    loading: boolean;
    error: string | null;

    // Actions
    refreshExtensions: () => Promise<void>;
    registerExtension: (manifestUrl: string, signatureAlgorithm?: string, allowedTenantIds?: string[]) => Promise<Extension>;
    installExtension: (extensionId: string, permissions: string[], settings?: Record<string, unknown>) => Promise<void>;
    uninstallExtension: (extensionId: string) => Promise<void>;
    updateExtensionSettings: (extensionId: string, enabled?: boolean, settings?: Record<string, unknown>) => Promise<void>;
    updateExtensionAccess: (extensionId: string, allowedTenantIds: string[]) => Promise<void>;
    validateManifest: (manifestUrl: string) => Promise<ExtensionManifest>;
    
    // Automation
    getAutomationJobs: (extensionId: string) => Promise<AutomationJob[]>;
    createAutomationJob: (extensionId: string, name: string, cronExpression: string, config?: Record<string, unknown>) => Promise<AutomationJob>;
    triggerAutomation: (jobId: string) => Promise<void>;
}

const ExtensionContext = createContext<ExtensionContextType | null>(null);

const API_URL = import.meta.env.VITE_API_URL || '';

export function ExtensionProvider({ children }: { children: ReactNode }) {
    const { token } = useAuth();
    const [extensions, setExtensions] = useState<Extension[]>([]);
    const [installedExtensions, setInstalledExtensions] = useState<InstalledExtension[]>([]);
    const [uiComponents, setUiComponents] = useState<UIExtensionComponents>({
        sidebar: [],
        buttons: [],
        components: [],
    });
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const authHeaders = useCallback(() => ({
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json',
    }), [token]);

    // Fetch all extensions data
    const refreshExtensions = useCallback(async () => {
        if (!token) return;

        setLoading(true);
        setError(null);

        try {
            const [extensionsRes, installedRes, uiRes] = await Promise.all([
                fetch(`${API_URL}/api/extensions/list`, { headers: authHeaders() }),
                fetch(`${API_URL}/api/extensions/installed`, { headers: authHeaders() }),
                fetch(`${API_URL}/api/extensions/ui`, { headers: authHeaders() }),
            ]);

            if (extensionsRes.ok) {
                const data = await extensionsRes.json();
                setExtensions(data);
            }

            if (installedRes.ok) {
                const data = await installedRes.json();
                setInstalledExtensions(data);
            }

            if (uiRes.ok) {
                const data = await uiRes.json();
                setUiComponents(data);
            }
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to fetch extensions');
        } finally {
            setLoading(false);
        }
    }, [token, authHeaders]);

    // Register a new extension
    const registerExtension = useCallback(async (
        manifestUrl: string,
        signatureAlgorithm: string = 'hmac_sha256',
        allowedTenantIds?: string[]
    ): Promise<Extension> => {
        const response = await fetch(`${API_URL}/api/extensions/register`, {
            method: 'POST',
            headers: authHeaders(),
            body: JSON.stringify({
                name: '',
                slug: '',
                manifest_url: manifestUrl,
                signature_algorithm: signatureAlgorithm,
                allowed_tenant_ids: allowedTenantIds,
            }),
        });

        if (!response.ok) {
            const error = await response.text();
            throw new Error(error || 'Failed to register extension');
        }

        const data = await response.json();
        await refreshExtensions();
        return data.extension;
    }, [authHeaders, refreshExtensions]);

    // Install an extension
    const installExtension = useCallback(async (
        extensionId: string,
        permissions: string[],
        settings?: Record<string, unknown>
    ): Promise<void> => {
        const response = await fetch(`${API_URL}/api/extensions/install/${extensionId}`, {
            method: 'POST',
            headers: authHeaders(),
            body: JSON.stringify({
                extension_id: extensionId,
                permissions,
                settings,
            }),
        });

        if (!response.ok) {
            const error = await response.text();
            throw new Error(error || 'Failed to install extension');
        }

        await refreshExtensions();
    }, [authHeaders, refreshExtensions]);

    // Uninstall an extension
    const uninstallExtension = useCallback(async (extensionId: string): Promise<void> => {
        const response = await fetch(`${API_URL}/api/extensions/${extensionId}`, {
            method: 'DELETE',
            headers: authHeaders(),
        });

        if (!response.ok) {
            const error = await response.text();
            throw new Error(error || 'Failed to uninstall extension');
        }

        await refreshExtensions();
    }, [authHeaders, refreshExtensions]);

    // Update extension settings
    const updateExtensionSettings = useCallback(async (
        extensionId: string,
        enabled?: boolean,
        settings?: Record<string, unknown>
    ): Promise<void> => {
        const response = await fetch(`${API_URL}/api/extensions/${extensionId}/settings`, {
            method: 'PUT',
            headers: authHeaders(),
            body: JSON.stringify({ enabled, settings }),
        });

        if (!response.ok) {
            const error = await response.text();
            throw new Error(error || 'Failed to update extension settings');
        }

        await refreshExtensions();
    }, [authHeaders, refreshExtensions]);

    // Update which companies can access an extension (owner only)
    const updateExtensionAccess = useCallback(async (
        extensionId: string,
        allowedTenantIds: string[]
    ): Promise<void> => {
        const response = await fetch(`${API_URL}/api/extensions/${extensionId}/access`, {
            method: 'PUT',
            headers: authHeaders(),
            body: JSON.stringify({ allowed_tenant_ids: allowedTenantIds }),
        });

        if (!response.ok) {
            const error = await response.text();
            throw new Error(error || 'Failed to update extension access');
        }

        await refreshExtensions();
    }, [authHeaders, refreshExtensions]);

    // Validate a manifest URL
    const validateManifest = useCallback(async (manifestUrl: string): Promise<ExtensionManifest> => {
        const response = await fetch(`${API_URL}/api/extensions/validate-manifest`, {
            method: 'POST',
            headers: authHeaders(),
            body: JSON.stringify({ manifest_url: manifestUrl }),
        });

        if (!response.ok) {
            const error = await response.text();
            throw new Error(error || 'Invalid manifest');
        }

        const data = await response.json();
        return data.manifest;
    }, [authHeaders]);

    // Get automation jobs for an extension
    const getAutomationJobs = useCallback(async (extensionId: string): Promise<AutomationJob[]> => {
        const response = await fetch(`${API_URL}/api/extensions/${extensionId}/jobs`, {
            headers: authHeaders(),
        });

        if (!response.ok) {
            throw new Error('Failed to fetch automation jobs');
        }

        return response.json();
    }, [authHeaders]);

    // Create an automation job
    const createAutomationJob = useCallback(async (
        extensionId: string,
        name: string,
        cronExpression: string,
        config?: Record<string, unknown>
    ): Promise<AutomationJob> => {
        const response = await fetch(`${API_URL}/api/extensions/${extensionId}/jobs`, {
            method: 'POST',
            headers: authHeaders(),
            body: JSON.stringify({
                extension_id: extensionId,
                name,
                cron_expression: cronExpression,
                config,
            }),
        });

        if (!response.ok) {
            const error = await response.text();
            throw new Error(error || 'Failed to create automation job');
        }

        const data = await response.json();
        return data.job;
    }, [authHeaders]);

    // Trigger an automation job
    const triggerAutomation = useCallback(async (jobId: string): Promise<void> => {
        const response = await fetch(`${API_URL}/api/extensions/trigger/automation/${jobId}`, {
            method: 'POST',
            headers: authHeaders(),
        });

        if (!response.ok) {
            const error = await response.text();
            throw new Error(error || 'Failed to trigger automation');
        }
    }, [authHeaders]);

    // Fetch extensions on mount and when token changes
    useEffect(() => {
        if (token) {
            refreshExtensions();
        }
    }, [token, refreshExtensions]);

    const value: ExtensionContextType = {
        extensions,
        installedExtensions,
        uiComponents,
        loading,
        error,
        refreshExtensions,
        registerExtension,
        installExtension,
        uninstallExtension,
        updateExtensionSettings,
        updateExtensionAccess,
        validateManifest,
        getAutomationJobs,
        createAutomationJob,
        triggerAutomation,
    };

    return (
        <ExtensionContext.Provider value={value}>
            {children}
        </ExtensionContext.Provider>
    );
}

export function useExtensions() {
    const context = useContext(ExtensionContext);
    if (!context) {
        throw new Error('useExtensions must be used within an ExtensionProvider');
    }
    return context;
}

