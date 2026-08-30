import { createContext, useContext, useState, useEffect, useCallback, ReactNode } from 'react';
import { useAuth, useAuthFetch } from './AuthContext';

export interface GlobalSettings {
    date_format: string;
    time_format: '12h' | '24h';
    timezone: string;
    footer_attribution: string;
    footer_disclaimer: string;
    app_name: string;
    logo_url: string | null;
    favicon_url: string | null;
    // Page content
    tos_content: string;
    privacy_content: string;
    help_content: string;
    // System
    maintenance_mode: boolean;
    maintenance_message: string;
    // Version & Updates
    github_repo: string | null;
}

const defaultSettings: GlobalSettings = {
    date_format: 'MM/DD/YYYY',
    time_format: '12h',
    timezone: 'America/New_York',
    footer_attribution: 'An open source project by ClovaLink.org',
    footer_disclaimer: 'ClovaLink is provided "as is" without warranty of any kind. The authors and contributors are not liable for any damages arising from use of this software.',
    app_name: 'ClovaLink',
    logo_url: null,
    favicon_url: null,
    // Page content defaults
    tos_content: '',
    privacy_content: '',
    help_content: '',
    // System defaults
    maintenance_mode: false,
    maintenance_message: 'We are currently performing scheduled maintenance. Please check back soon.',
    // Version & Updates
    github_repo: null,
};

interface GlobalSettingsContextType {
    settings: GlobalSettings;
    isLoading: boolean;
    error: string | null;
    updateSettings: (updates: Partial<GlobalSettings>) => Promise<boolean>;
    uploadLogo: (file: File) => Promise<string | null>;
    deleteLogo: () => Promise<boolean>;
    uploadFavicon: (file: File) => Promise<string | null>;
    deleteFavicon: () => Promise<boolean>;
    refreshSettings: () => Promise<void>;
    formatDate: (date: Date | string | null | undefined) => string;
    formatTime: (date: Date | string | null | undefined) => string;
    formatDateTime: (date: Date | string | null | undefined) => string;
}

const GlobalSettingsContext = createContext<GlobalSettingsContextType | undefined>(undefined);

// Helper to unwrap potentially double-quoted JSON strings
const unwrapValue = (value: any, fallback: string): string => {
    if (value === null || value === undefined) return fallback;
    // If value is a string that looks like it has extra quotes, unwrap them
    if (typeof value === 'string') {
        // Remove surrounding quotes if present (from JSONB storage)
        const trimmed = value.trim();
        if (trimmed.startsWith('"') && trimmed.endsWith('"')) {
            return trimmed.slice(1, -1);
        }
        return value;
    }
    return String(value);
};

export function GlobalSettingsProvider({ children }: { children: ReactNode }) {
    const [settings, setSettings] = useState<GlobalSettings>(defaultSettings);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const { isAuthenticated } = useAuth();
    const authFetch = useAuthFetch();

    const parseSettings = (data: any): GlobalSettings => ({
        date_format: unwrapValue(data.date_format, defaultSettings.date_format),
        time_format: unwrapValue(data.time_format, defaultSettings.time_format) as '12h' | '24h',
        timezone: unwrapValue(data.timezone, defaultSettings.timezone),
        footer_attribution: unwrapValue(data.footer_attribution, defaultSettings.footer_attribution),
        footer_disclaimer: unwrapValue(data.footer_disclaimer, defaultSettings.footer_disclaimer),
        app_name: unwrapValue(data.app_name, defaultSettings.app_name),
        logo_url: data.logo_url ? unwrapValue(data.logo_url, '') : null,
        favicon_url: data.favicon_url ? unwrapValue(data.favicon_url, '') : null,
        // Page content
        tos_content: unwrapValue(data.tos_content, defaultSettings.tos_content),
        privacy_content: unwrapValue(data.privacy_content, defaultSettings.privacy_content),
        help_content: unwrapValue(data.help_content, defaultSettings.help_content),
        // System
        maintenance_mode: data.maintenance_mode === true || data.maintenance_mode === 'true',
        maintenance_message: unwrapValue(data.maintenance_message, defaultSettings.maintenance_message),
        // Version & Updates
        github_repo: data.github_repo ? unwrapValue(data.github_repo, '') : null,
    });

    const fetchSettings = useCallback(async () => {
        if (!isAuthenticated) {
            setIsLoading(false);
            return;
        }

        try {
            const response = await authFetch('/api/global-settings');
            if (response.ok) {
                const data = await response.json();
                setSettings(parseSettings(data));
                setError(null);
            }
        } catch (err) {
            console.error('Failed to fetch global settings:', err);
            setError('Failed to load settings');
        } finally {
            setIsLoading(false);
        }
    }, [authFetch, isAuthenticated]);

    useEffect(() => {
        fetchSettings();
    }, [fetchSettings]);

    // Update favicon dynamically when settings change
    useEffect(() => {
        const link = document.querySelector("link[rel~='icon']") as HTMLLinkElement;
        if (link) {
            if (settings.favicon_url) {
                link.href = settings.favicon_url;
            } else {
                // Reset to default
                link.href = '/vite.svg';
            }
        }
    }, [settings.favicon_url]);

    // Update document title dynamically when app_name changes
    useEffect(() => {
        if (settings.app_name) {
            document.title = settings.app_name;
        }
    }, [settings.app_name]);

    const updateSettings = useCallback(async (updates: Partial<GlobalSettings>): Promise<boolean> => {
        try {
            const settingsArray = Object.entries(updates)
                .filter(([key]) => key !== 'logo_url' && key !== 'favicon_url') // Logo/Favicon are handled separately
                .map(([key, value]) => ({
                    key,
                    value: value === null ? '' : value, // Convert null to empty string for storage
                }));

            const response = await authFetch('/api/global-settings', {
                method: 'PUT',
                body: JSON.stringify({ settings: settingsArray }),
            });

            if (response.ok) {
                const data = await response.json();
                setSettings(parseSettings(data));
                return true;
            }
            return false;
        } catch (err) {
            console.error('Failed to update global settings:', err);
            return false;
        }
    }, [authFetch]);

    const uploadLogo = useCallback(async (file: File): Promise<string | null> => {
        try {
            const formData = new FormData();
            formData.append('logo', file);

            const response = await fetch('/api/global-settings/logo', {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${localStorage.getItem('auth_token')}`,
                },
                body: formData,
            });

            if (response.ok) {
                const data = await response.json();
                setSettings(prev => ({ ...prev, logo_url: data.logo_url }));
                return data.logo_url;
            }
            return null;
        } catch (err) {
            console.error('Failed to upload logo:', err);
            return null;
        }
    }, []);

    const deleteLogo = useCallback(async (): Promise<boolean> => {
        try {
            const response = await authFetch('/api/global-settings/logo', {
                method: 'DELETE',
            });

            if (response.ok) {
                setSettings(prev => ({ ...prev, logo_url: null }));
                return true;
            }
            return false;
        } catch (err) {
            console.error('Failed to delete logo:', err);
            return false;
        }
    }, [authFetch]);

    const uploadFavicon = useCallback(async (file: File): Promise<string | null> => {
        try {
            const formData = new FormData();
            formData.append('favicon', file);

            const response = await fetch('/api/global-settings/favicon', {
                method: 'POST',
                headers: {
                    'Authorization': `Bearer ${localStorage.getItem('auth_token')}`,
                },
                body: formData,
            });

            if (response.ok) {
                const data = await response.json();
                setSettings(prev => ({ ...prev, favicon_url: data.favicon_url }));
                return data.favicon_url;
            }
            return null;
        } catch (err) {
            console.error('Failed to upload favicon:', err);
            return null;
        }
    }, []);

    const deleteFavicon = useCallback(async (): Promise<boolean> => {
        try {
            const response = await authFetch('/api/global-settings/favicon', {
                method: 'DELETE',
            });

            if (response.ok) {
                setSettings(prev => ({ ...prev, favicon_url: null }));
                return true;
            }
            return false;
        } catch (err) {
            console.error('Failed to delete favicon:', err);
            return false;
        }
    }, [authFetch]);

    const formatDate = useCallback((date: Date | string | null | undefined): string => {
        if (!date) return 'N/A';
        const d = typeof date === 'string' ? new Date(date) : date;
        if (isNaN(d.getTime())) return 'Invalid date';
        const format = settings.date_format;
        
        const day = d.getDate().toString().padStart(2, '0');
        const month = (d.getMonth() + 1).toString().padStart(2, '0');
        const year = d.getFullYear().toString();
        
        switch (format) {
            case 'DD/MM/YYYY':
                return `${day}/${month}/${year}`;
            case 'YYYY-MM-DD':
                return `${year}-${month}-${day}`;
            case 'MM/DD/YYYY':
            default:
                return `${month}/${day}/${year}`;
        }
    }, [settings.date_format]);

    const formatTime = useCallback((date: Date | string | null | undefined): string => {
        if (!date) return '';
        const d = typeof date === 'string' ? new Date(date) : date;
        if (isNaN(d.getTime())) return '';
        
        if (settings.time_format === '24h') {
            return d.toLocaleTimeString('en-US', { 
                hour: '2-digit', 
                minute: '2-digit',
                hour12: false 
            });
        }
        
        return d.toLocaleTimeString('en-US', { 
            hour: 'numeric', 
            minute: '2-digit',
            hour12: true 
        });
    }, [settings.time_format]);

    const formatDateTime = useCallback((date: Date | string | null | undefined): string => {
        if (!date) return 'N/A';
        return `${formatDate(date)} ${formatTime(date)}`;
    }, [formatDate, formatTime]);

    return (
        <GlobalSettingsContext.Provider value={{
            settings,
            isLoading,
            error,
            updateSettings,
            uploadLogo,
            deleteLogo,
            uploadFavicon,
            deleteFavicon,
            refreshSettings: fetchSettings,
            formatDate,
            formatTime,
            formatDateTime,
        }}>
            {children}
        </GlobalSettingsContext.Provider>
    );
}

export function useGlobalSettings() {
    const context = useContext(GlobalSettingsContext);
    if (context === undefined) {
        throw new Error('useGlobalSettings must be used within a GlobalSettingsProvider');
    }
    return context;
}

