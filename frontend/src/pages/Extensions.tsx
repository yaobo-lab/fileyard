import { useState } from 'react';
import { Navigate } from 'react-router-dom';
import { 
    Puzzle, Plus, Settings, Trash2, Power, PowerOff, 
    ExternalLink, Clock, FileCode, Zap, RefreshCw,
    CheckCircle, XCircle, AlertCircle, ChevronRight, ShieldX,
    Crown, Users, Building2
} from 'lucide-react';
import clsx from 'clsx';
import { useExtensions, InstalledExtension, Extension } from '../hooks/useExtensions';
import { useAuth } from '../context/AuthContext';
import { useTenant } from '../context/TenantContext';
import { useGlobalSettings } from '../context/GlobalSettingsContext';

// Permission descriptions
const PERMISSION_LABELS: Record<string, { label: string; description: string }> = {
    'read:files': { label: 'Read Files', description: 'Access file metadata and contents' },
    'write:files': { label: 'Write Files', description: 'Upload, modify, and delete files' },
    'read:company': { label: 'Read Company', description: 'Access company information' },
    'read:employees': { label: 'Read Employees', description: 'Access employee data' },
    'automation:run': { label: 'Run Automation', description: 'Execute scheduled tasks' },
    'file_processor:run': { label: 'Process Files', description: 'Process uploaded files' },
};

// Extension type icons and colors
const TYPE_CONFIG: Record<string, { icon: typeof Puzzle; color: string; label: string }> = {
    ui: { icon: Puzzle, color: 'text-blue-500 bg-blue-100 dark:bg-blue-900/30', label: 'UI Extension' },
    file_processor: { icon: FileCode, color: 'text-green-500 bg-green-100 dark:bg-green-900/30', label: 'File Processor' },
    automation: { icon: Zap, color: 'text-amber-500 bg-amber-100 dark:bg-amber-900/30', label: 'Automation' },
};

export function Extensions() {
    const { user } = useAuth();
    const { companies } = useTenant();
    const { formatDate } = useGlobalSettings();
    const {
        extensions,
        installedExtensions,
        loading,
        error,
        refreshExtensions,
        registerExtension,
        installExtension,
        uninstallExtension,
        updateExtensionSettings,
        updateExtensionAccess,
        validateManifest,
    } = useExtensions();

    // Only SuperAdmins can access this page
    if (!user || user.role !== 'SuperAdmin') {
        return (
            <div className="max-w-2xl mx-auto py-12">
                <div className="text-center bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-8">
                    <ShieldX className="h-16 w-16 mx-auto text-red-400" />
                    <h2 className="mt-4 text-xl font-semibold text-gray-900 dark:text-white">
                        Access Denied
                    </h2>
                    <p className="mt-2 text-gray-500 dark:text-gray-400">
                        Only SuperAdmins can manage extensions.
                    </p>
                </div>
            </div>
        );
    }
    const [showRegisterModal, setShowRegisterModal] = useState(false);
    const [showInstallModal, setShowInstallModal] = useState(false);
    const [showAccessModal, setShowAccessModal] = useState(false);
    const [selectedExtension, setSelectedExtension] = useState<Extension | null>(null);
    const [manifestUrl, setManifestUrl] = useState('');
    const [validatingManifest, setValidatingManifest] = useState(false);
    const [manifestError, setManifestError] = useState<string | null>(null);
    const [selectedPermissions, setSelectedPermissions] = useState<string[]>([]);
    const [selectedCompanies, setSelectedCompanies] = useState<string[]>([]);
    const [signatureAlgorithm, setSignatureAlgorithm] = useState<'hmac_sha256' | 'ed25519'>('hmac_sha256');
    const [activeTab, setActiveTab] = useState<'installed' | 'available'>('installed');

    // Get available (not installed) extensions
    const availableExtensions = extensions.filter(
        ext => !installedExtensions.some(inst => inst.extension_id === ext.id)
    );

    // Handle register new extension
    const handleRegister = async () => {
        if (!manifestUrl.trim()) return;

        setValidatingManifest(true);
        setManifestError(null);

        try {
            // First validate
            await validateManifest(manifestUrl);
            // Then register with selected companies
            await registerExtension(
                manifestUrl, 
                signatureAlgorithm,
                selectedCompanies.length > 0 ? selectedCompanies : undefined
            );
            setShowRegisterModal(false);
            setManifestUrl('');
            setSelectedCompanies([]);
        } catch (err) {
            setManifestError(err instanceof Error ? err.message : 'Failed to register extension');
        } finally {
            setValidatingManifest(false);
        }
    };

    // Handle update company access
    const handleUpdateAccess = async () => {
        if (!selectedExtension) return;

        try {
            await updateExtensionAccess(selectedExtension.id, selectedCompanies);
            setShowAccessModal(false);
            setSelectedExtension(null);
            setSelectedCompanies([]);
        } catch (err) {
            console.error('Failed to update access:', err);
        }
    };

    // Open access modal for an extension
    const openAccessModal = (ext: Extension) => {
        setSelectedExtension(ext);
        setSelectedCompanies(ext.allowed_tenant_ids || []);
        setShowAccessModal(true);
    };

    // Handle install extension
    const handleInstall = async () => {
        if (!selectedExtension) return;

        try {
            await installExtension(selectedExtension.id, selectedPermissions);
            setShowInstallModal(false);
            setSelectedExtension(null);
            setSelectedPermissions([]);
        } catch (err) {
            console.error('Install error:', err);
        }
    };

    // Handle uninstall
    const handleUninstall = async (extensionId: string) => {
        if (!confirm('Are you sure you want to uninstall this extension?')) return;
        await uninstallExtension(extensionId);
    };

    // Handle toggle enabled
    const handleToggleEnabled = async (ext: InstalledExtension) => {
        await updateExtensionSettings(ext.extension_id, !ext.enabled);
    };

    // Open install modal
    const openInstallModal = (ext: Extension) => {
        setSelectedExtension(ext);
        setSelectedPermissions(ext.manifest?.permissions || []);
        setShowInstallModal(true);
    };

    return (
        <div className="max-w-6xl mx-auto">
            {/* Header */}
            <div className="flex items-center justify-between mb-8">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Extensions</h1>
                    <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
                        Manage third-party integrations and automations
                    </p>
                </div>
                <div className="flex items-center gap-3">
                    <button
                        onClick={() => refreshExtensions()}
                        className="p-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800"
                        title="Refresh"
                    >
                        <RefreshCw className={clsx("h-5 w-5", loading && "animate-spin")} />
                    </button>
                    <button
                        onClick={() => setShowRegisterModal(true)}
                        className="inline-flex items-center px-4 py-2 bg-primary-600 text-white text-sm font-medium rounded-lg hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary-500"
                    >
                        <Plus className="h-4 w-4 mr-2" />
                        Register Extension
                    </button>
                </div>
            </div>

            {/* Error display */}
            {error && (
                <div className="mb-6 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
                    <div className="flex items-center text-red-700 dark:text-red-400">
                        <AlertCircle className="h-5 w-5 mr-2" />
                        {error}
                    </div>
                </div>
            )}

            {/* Tabs */}
            <div className="border-b border-gray-200 dark:border-gray-700 mb-6">
                <nav className="-mb-px flex space-x-8">
                    <button
                        onClick={() => setActiveTab('installed')}
                        className={clsx(
                            "py-4 px-1 border-b-2 font-medium text-sm",
                            activeTab === 'installed'
                                ? "border-primary-500 text-primary-600 dark:text-primary-400"
                                : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300"
                        )}
                    >
                        Installed ({installedExtensions.length})
                    </button>
                    <button
                        onClick={() => setActiveTab('available')}
                        className={clsx(
                            "py-4 px-1 border-b-2 font-medium text-sm",
                            activeTab === 'available'
                                ? "border-primary-500 text-primary-600 dark:text-primary-400"
                                : "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 dark:text-gray-400 dark:hover:text-gray-300"
                        )}
                    >
                        Available ({availableExtensions.length})
                    </button>
                </nav>
            </div>

            {/* Content */}
            {activeTab === 'installed' ? (
                <div className="space-y-4">
                    {installedExtensions.length === 0 ? (
                        <div className="text-center py-12 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                            <Puzzle className="h-12 w-12 mx-auto text-gray-400 dark:text-gray-500" />
                            <h3 className="mt-4 text-lg font-medium text-gray-900 dark:text-white">No extensions installed</h3>
                            <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
                                Register and install extensions to enhance your workflow
                            </p>
                        </div>
                    ) : (
                        installedExtensions.map((ext) => (
                            <InstalledExtensionCard
                                key={ext.installation_id}
                                extension={ext}
                                onToggle={() => handleToggleEnabled(ext)}
                                onUninstall={() => handleUninstall(ext.extension_id)}
                            />
                        ))
                    )}
                </div>
            ) : (
                <div className="space-y-4">
                    {availableExtensions.length === 0 ? (
                        <div className="text-center py-12 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                            <CheckCircle className="h-12 w-12 mx-auto text-green-400" />
                            <h3 className="mt-4 text-lg font-medium text-gray-900 dark:text-white">All extensions installed</h3>
                            <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
                                Register a new extension to add more functionality
                            </p>
                        </div>
                    ) : (
                        availableExtensions.map((ext) => (
                            <AvailableExtensionCard
                                key={ext.id}
                                extension={ext}
                                onInstall={() => openInstallModal(ext)}
                                onManageAccess={ext.is_owner ? () => openAccessModal(ext) : undefined}
                            />
                        ))
                    )}
                </div>
            )}

            {/* Register Modal */}
            {showRegisterModal && (
                <div className="fixed inset-0 z-50 overflow-y-auto">
                    <div className="flex min-h-full items-center justify-center p-4">
                        <div className="fixed inset-0 bg-black/50" onClick={() => setShowRegisterModal(false)} />
                        <div className="relative bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-lg w-full p-6">
                            <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">
                                Register New Extension
                            </h2>
                            
                            <div className="space-y-4">
                                <div>
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                        Manifest URL
                                    </label>
                                    <input
                                        type="url"
                                        value={manifestUrl}
                                        onChange={(e) => setManifestUrl(e.target.value)}
                                        placeholder="https://example.com/extension/manifest.json"
                                        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                                    />
                                    <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                                        URL to the extension's manifest.json file
                                    </p>
                                </div>

                                <div>
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                        Signature Algorithm
                                    </label>
                                    <select
                                        value={signatureAlgorithm}
                                        onChange={(e) => setSignatureAlgorithm(e.target.value as 'hmac_sha256' | 'ed25519')}
                                        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                                    >
                                        <option value="hmac_sha256">HMAC-SHA256 (Recommended)</option>
                                        <option value="ed25519">Ed25519</option>
                                    </select>
                                </div>

                                <div>
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                        Companies with Access
                                    </label>
                                    <p className="text-xs text-gray-500 dark:text-gray-400 mb-2">
                                        Select which companies can install this extension. Leave empty for only your company.
                                    </p>
                                    <div className="max-h-40 overflow-y-auto border border-gray-200 dark:border-gray-600 rounded-lg">
                                        {companies.filter(c => c.status === 'active').map((company) => (
                                            <label
                                                key={company.id}
                                                className="flex items-center gap-2 px-3 py-2 hover:bg-gray-50 dark:hover:bg-gray-700 cursor-pointer"
                                            >
                                                <input
                                                    type="checkbox"
                                                    checked={selectedCompanies.includes(company.id)}
                                                    onChange={(e) => {
                                                        if (e.target.checked) {
                                                            setSelectedCompanies([...selectedCompanies, company.id]);
                                                        } else {
                                                            setSelectedCompanies(selectedCompanies.filter(id => id !== company.id));
                                                        }
                                                    }}
                                                    className="h-4 w-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                                                />
                                                <span className="text-sm text-gray-700 dark:text-gray-300">{company.name}</span>
                                            </label>
                                        ))}
                                    </div>
                                </div>

                                {manifestError && (
                                    <div className="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
                                        <div className="flex items-center text-red-700 dark:text-red-400 text-sm">
                                            <XCircle className="h-4 w-4 mr-2 flex-shrink-0" />
                                            {manifestError}
                                        </div>
                                    </div>
                                )}
                            </div>

                            <div className="mt-6 flex justify-end gap-3">
                                <button
                                    onClick={() => setShowRegisterModal(false)}
                                    className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg"
                                >
                                    Cancel
                                </button>
                                <button
                                    onClick={handleRegister}
                                    disabled={!manifestUrl.trim() || validatingManifest}
                                    className="px-4 py-2 text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 rounded-lg disabled:opacity-50 disabled:cursor-not-allowed"
                                >
                                    {validatingManifest ? 'Validating...' : 'Register Extension'}
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            )}

            {/* Install Modal */}
            {showInstallModal && selectedExtension && (
                <div className="fixed inset-0 z-50 overflow-y-auto">
                    <div className="flex min-h-full items-center justify-center p-4">
                        <div className="fixed inset-0 bg-black/50" onClick={() => setShowInstallModal(false)} />
                        <div className="relative bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-lg w-full p-6">
                            <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">
                                Install {selectedExtension.name}
                            </h2>

                            <div className="space-y-4">
                                <p className="text-sm text-gray-600 dark:text-gray-400">
                                    {selectedExtension.description || 'No description provided'}
                                </p>

                                <div>
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                        Requested Permissions
                                    </label>
                                    <div className="space-y-2">
                                        {(selectedExtension.manifest?.permissions || []).map((perm) => (
                                            <label key={perm} className="flex items-start gap-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg cursor-pointer">
                                                <input
                                                    type="checkbox"
                                                    checked={selectedPermissions.includes(perm)}
                                                    onChange={(e) => {
                                                        if (e.target.checked) {
                                                            setSelectedPermissions([...selectedPermissions, perm]);
                                                        } else {
                                                            setSelectedPermissions(selectedPermissions.filter(p => p !== perm));
                                                        }
                                                    }}
                                                    className="mt-0.5 h-4 w-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                                                />
                                                <div>
                                                    <div className="font-medium text-gray-900 dark:text-white text-sm">
                                                        {PERMISSION_LABELS[perm]?.label || perm}
                                                    </div>
                                                    <div className="text-xs text-gray-500 dark:text-gray-400">
                                                        {PERMISSION_LABELS[perm]?.description || ''}
                                                    </div>
                                                </div>
                                            </label>
                                        ))}
                                    </div>
                                </div>
                            </div>

                            <div className="mt-6 flex justify-end gap-3">
                                <button
                                    onClick={() => setShowInstallModal(false)}
                                    className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg"
                                >
                                    Cancel
                                </button>
                                <button
                                    onClick={handleInstall}
                                    className="px-4 py-2 text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 rounded-lg"
                                >
                                    Install Extension
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            )}

            {/* Access Modal - Manage which companies can access */}
            {showAccessModal && selectedExtension && (
                <div className="fixed inset-0 z-50 overflow-y-auto">
                    <div className="flex min-h-full items-center justify-center p-4">
                        <div className="fixed inset-0 bg-black/50" onClick={() => setShowAccessModal(false)} />
                        <div className="relative bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-lg w-full p-6">
                            <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">
                                Manage Company Access
                            </h2>
                            <p className="text-sm text-gray-600 dark:text-gray-400 mb-4">
                                Select which companies can install <strong>{selectedExtension.name}</strong>
                            </p>

                            <div className="max-h-64 overflow-y-auto border border-gray-200 dark:border-gray-600 rounded-lg">
                                {companies.filter(c => c.status === 'active').map((company) => (
                                    <label
                                        key={company.id}
                                        className="flex items-center gap-3 px-4 py-3 hover:bg-gray-50 dark:hover:bg-gray-700 cursor-pointer border-b border-gray-100 dark:border-gray-700 last:border-0"
                                    >
                                        <input
                                            type="checkbox"
                                            checked={selectedCompanies.includes(company.id)}
                                            onChange={(e) => {
                                                if (e.target.checked) {
                                                    setSelectedCompanies([...selectedCompanies, company.id]);
                                                } else {
                                                    setSelectedCompanies(selectedCompanies.filter(id => id !== company.id));
                                                }
                                            }}
                                            className="h-4 w-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                                        />
                                        <Building2 className="h-4 w-4 text-gray-400" />
                                        <span className="text-sm text-gray-700 dark:text-gray-300">{company.name}</span>
                                    </label>
                                ))}
                            </div>

                            <p className="mt-3 text-xs text-gray-500 dark:text-gray-400">
                                {selectedCompanies.length === 0 
                                    ? 'No companies selected - only your company will have access'
                                    : `${selectedCompanies.length} companies will have access`}
                            </p>

                            <div className="mt-6 flex justify-end gap-3">
                                <button
                                    onClick={() => setShowAccessModal(false)}
                                    className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg"
                                >
                                    Cancel
                                </button>
                                <button
                                    onClick={handleUpdateAccess}
                                    className="px-4 py-2 text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 rounded-lg"
                                >
                                    Save Access
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}

// Installed extension card component
function InstalledExtensionCard({
    extension,
    onToggle,
    onUninstall,
}: {
    extension: InstalledExtension;
    onToggle: () => void;
    onUninstall: () => void;
}) {
    const { formatDate } = useGlobalSettings();
    const typeConfig = TYPE_CONFIG[extension.type] || TYPE_CONFIG.ui;
    const TypeIcon = typeConfig.icon;

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4">
            <div className="flex items-start justify-between">
                <div className="flex items-start gap-4">
                    <div className={clsx("p-2.5 rounded-lg", typeConfig.color)}>
                        <TypeIcon className="h-5 w-5" />
                    </div>
                    <div>
                        <div className="flex items-center gap-2">
                            <h3 className="font-medium text-gray-900 dark:text-white">{extension.name}</h3>
                            <span className="px-2 py-0.5 text-xs font-medium bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded">
                                v{extension.version}
                            </span>
                            {extension.enabled ? (
                                <span className="px-2 py-0.5 text-xs font-medium bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400 rounded">
                                    Active
                                </span>
                            ) : (
                                <span className="px-2 py-0.5 text-xs font-medium bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded">
                                    Disabled
                                </span>
                            )}
                        </div>
                        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
                            {extension.description || 'No description'}
                        </p>
                        <div className="mt-2 flex items-center gap-4 text-xs text-gray-500 dark:text-gray-400">
                            <span className="flex items-center gap-1">
                                <Clock className="h-3.5 w-3.5" />
                                Installed {formatDate(extension.installed_at)}
                            </span>
                            <span>{extension.permissions.length} permissions</span>
                        </div>
                    </div>
                </div>

                <div className="flex items-center gap-2">
                    <button
                        onClick={onToggle}
                        className={clsx(
                            "p-2 rounded-lg transition-colors",
                            extension.enabled
                                ? "text-green-600 hover:bg-green-50 dark:hover:bg-green-900/20"
                                : "text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
                        )}
                        title={extension.enabled ? 'Disable' : 'Enable'}
                    >
                        {extension.enabled ? <Power className="h-5 w-5" /> : <PowerOff className="h-5 w-5" />}
                    </button>
                    <button
                        onClick={onUninstall}
                        className="p-2 text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
                        title="Uninstall"
                    >
                        <Trash2 className="h-5 w-5" />
                    </button>
                </div>
            </div>
        </div>
    );
}

// Available extension card component
function AvailableExtensionCard({
    extension,
    onInstall,
    onManageAccess,
}: {
    extension: Extension;
    onInstall: () => void;
    onManageAccess?: () => void;
}) {
    const typeConfig = TYPE_CONFIG[extension.type] || TYPE_CONFIG.ui;
    const TypeIcon = typeConfig.icon;

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-4">
            <div className="flex items-start justify-between">
                <div className="flex items-start gap-4">
                    <div className={clsx("p-2.5 rounded-lg", typeConfig.color)}>
                        <TypeIcon className="h-5 w-5" />
                    </div>
                    <div>
                        <div className="flex items-center gap-2">
                            <h3 className="font-medium text-gray-900 dark:text-white">{extension.name}</h3>
                            {extension.current_version && (
                                <span className="px-2 py-0.5 text-xs font-medium bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded">
                                    v{extension.current_version}
                                </span>
                            )}
                            <span className={clsx(
                                "px-2 py-0.5 text-xs font-medium rounded",
                                typeConfig.color
                            )}>
                                {typeConfig.label}
                            </span>
                            {extension.is_owner && (
                                <span className="px-2 py-0.5 text-xs font-medium bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400 rounded flex items-center gap-1">
                                    <Crown className="h-3 w-3" />
                                    Owner
                                </span>
                            )}
                        </div>
                        <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
                            {extension.description || 'No description'}
                        </p>
                        <div className="mt-2 flex flex-wrap items-center gap-2">
                            {extension.manifest?.permissions && extension.manifest.permissions.length > 0 && (
                                <>
                                    {extension.manifest.permissions.slice(0, 3).map((perm) => (
                                        <span
                                            key={perm}
                                            className="px-2 py-0.5 text-xs bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded"
                                        >
                                            {PERMISSION_LABELS[perm]?.label || perm}
                                        </span>
                                    ))}
                                    {extension.manifest.permissions.length > 3 && (
                                        <span className="px-2 py-0.5 text-xs bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded">
                                            +{extension.manifest.permissions.length - 3} more
                                        </span>
                                    )}
                                </>
                            )}
                            {extension.allowed_tenant_ids && extension.allowed_tenant_ids.length > 0 && (
                                <span className="px-2 py-0.5 text-xs bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400 rounded flex items-center gap-1">
                                    <Users className="h-3 w-3" />
                                    {extension.allowed_tenant_ids.length} companies
                                </span>
                            )}
                        </div>
                    </div>
                </div>

                <div className="flex items-center gap-2">
                    {extension.is_owner && onManageAccess && (
                        <button
                            onClick={onManageAccess}
                            className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                            title="Manage company access"
                        >
                            <Users className="h-4 w-4" />
                        </button>
                    )}
                    <button
                        onClick={onInstall}
                        className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-primary-600 dark:text-primary-400 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-lg transition-colors"
                    >
                        Install
                        <ChevronRight className="h-4 w-4 ml-1" />
                    </button>
                </div>
            </div>
        </div>
    );
}

export default Extensions;

