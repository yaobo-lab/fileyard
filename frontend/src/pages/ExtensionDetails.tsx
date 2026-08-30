import { useState, useEffect, useCallback } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { 
    ArrowLeft, Settings, Clock, Play, Pause, Trash2, Plus, 
    RefreshCw, CheckCircle, XCircle, AlertCircle, Calendar,
    Puzzle, FileCode, Zap, ShieldX
} from 'lucide-react';
import clsx from 'clsx';
import { useExtensions, InstalledExtension, AutomationJob } from '../hooks/useExtensions';
import { useAuth } from '../context/AuthContext';
import { useGlobalSettings } from '../context/GlobalSettingsContext';

const TYPE_CONFIG: Record<string, { icon: typeof Puzzle; color: string; label: string }> = {
    ui: { icon: Puzzle, color: 'text-blue-500 bg-blue-100 dark:bg-blue-900/30', label: 'UI Extension' },
    file_processor: { icon: FileCode, color: 'text-green-500 bg-green-100 dark:bg-green-900/30', label: 'File Processor' },
    automation: { icon: Zap, color: 'text-amber-500 bg-amber-100 dark:bg-amber-900/30', label: 'Automation' },
};

export function ExtensionDetails() {
    const { id } = useParams<{ id: string }>();
    const navigate = useNavigate();
    const { user } = useAuth();
    const { formatDate } = useGlobalSettings();
    const {
        installedExtensions,
        updateExtensionSettings,
        uninstallExtension,
        getAutomationJobs,
        createAutomationJob,
        triggerAutomation,
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
    const [extension, setExtension] = useState<InstalledExtension | null>(null);
    const [jobs, setJobs] = useState<AutomationJob[]>([]);
    const [loading, setLoading] = useState(true);
    const [showCreateJobModal, setShowCreateJobModal] = useState(false);
    const [newJobName, setNewJobName] = useState('');
    const [newJobCron, setNewJobCron] = useState('0 0 * * *');
    const [triggering, setTriggering] = useState<string | null>(null);

    // Find extension
    useEffect(() => {
        const ext = installedExtensions.find(e => e.extension_id === id);
        setExtension(ext || null);
        setLoading(false);
    }, [id, installedExtensions]);

    // Fetch automation jobs if applicable
    const fetchJobs = useCallback(async () => {
        if (!id || extension?.type !== 'automation') return;
        try {
            const fetchedJobs = await getAutomationJobs(id);
            setJobs(fetchedJobs);
        } catch (err) {
            console.error('Failed to fetch jobs:', err);
        }
    }, [id, extension?.type, getAutomationJobs]);

    useEffect(() => {
        fetchJobs();
    }, [fetchJobs]);

    // Handle create job
    const handleCreateJob = async () => {
        if (!id || !newJobName.trim() || !newJobCron.trim()) return;

        try {
            await createAutomationJob(id, newJobName, newJobCron);
            await fetchJobs();
            setShowCreateJobModal(false);
            setNewJobName('');
            setNewJobCron('0 0 * * *');
        } catch (err) {
            console.error('Failed to create job:', err);
        }
    };

    // Handle trigger job
    const handleTriggerJob = async (jobId: string) => {
        setTriggering(jobId);
        try {
            await triggerAutomation(jobId);
            await fetchJobs();
        } catch (err) {
            console.error('Failed to trigger job:', err);
        } finally {
            setTriggering(null);
        }
    };

    // Handle toggle enabled
    const handleToggleEnabled = async () => {
        if (!extension) return;
        await updateExtensionSettings(extension.extension_id, !extension.enabled);
    };

    // Handle uninstall
    const handleUninstall = async () => {
        if (!extension) return;
        if (!confirm('Are you sure you want to uninstall this extension?')) return;
        await uninstallExtension(extension.extension_id);
        navigate('/extensions');
    };

    if (loading) {
        return (
            <div className="flex items-center justify-center h-64">
                <RefreshCw className="h-8 w-8 text-gray-400 animate-spin" />
            </div>
        );
    }

    if (!extension) {
        return (
            <div className="max-w-4xl mx-auto">
                <div className="text-center py-12 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
                    <AlertCircle className="h-12 w-12 mx-auto text-gray-400" />
                    <h3 className="mt-4 text-lg font-medium text-gray-900 dark:text-white">Extension not found</h3>
                    <p className="mt-2 text-sm text-gray-500 dark:text-gray-400">
                        The extension you're looking for doesn't exist or isn't installed.
                    </p>
                    <button
                        onClick={() => navigate('/extensions')}
                        className="mt-4 inline-flex items-center px-4 py-2 text-sm font-medium text-primary-600 hover:text-primary-700"
                    >
                        <ArrowLeft className="h-4 w-4 mr-2" />
                        Back to Extensions
                    </button>
                </div>
            </div>
        );
    }

    const typeConfig = TYPE_CONFIG[extension.type] || TYPE_CONFIG.ui;
    const TypeIcon = typeConfig.icon;

    return (
        <div className="max-w-4xl mx-auto">
            {/* Back button */}
            <button
                onClick={() => navigate('/extensions')}
                className="mb-6 inline-flex items-center text-sm text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300"
            >
                <ArrowLeft className="h-4 w-4 mr-1" />
                Back to Extensions
            </button>

            {/* Header */}
            <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6 mb-6">
                <div className="flex items-start justify-between">
                    <div className="flex items-start gap-4">
                        <div className={clsx("p-3 rounded-lg", typeConfig.color)}>
                            <TypeIcon className="h-6 w-6" />
                        </div>
                        <div>
                            <div className="flex items-center gap-3">
                                <h1 className="text-2xl font-bold text-gray-900 dark:text-white">{extension.name}</h1>
                                <span className="px-2 py-0.5 text-sm font-medium bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded">
                                    v{extension.version}
                                </span>
                                {extension.enabled ? (
                                    <span className="px-2 py-0.5 text-sm font-medium bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400 rounded flex items-center gap-1">
                                        <CheckCircle className="h-3.5 w-3.5" />
                                        Active
                                    </span>
                                ) : (
                                    <span className="px-2 py-0.5 text-sm font-medium bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded flex items-center gap-1">
                                        <XCircle className="h-3.5 w-3.5" />
                                        Disabled
                                    </span>
                                )}
                            </div>
                            <p className="mt-2 text-gray-500 dark:text-gray-400">
                                {extension.description || 'No description provided'}
                            </p>
                            <div className="mt-3 flex items-center gap-4 text-sm text-gray-500 dark:text-gray-400">
                                <span className={clsx("px-2 py-0.5 rounded", typeConfig.color)}>
                                    {typeConfig.label}
                                </span>
                                <span className="flex items-center gap-1">
                                    <Clock className="h-4 w-4" />
                                    Installed {formatDate(extension.installed_at)}
                                </span>
                            </div>
                        </div>
                    </div>

                    <div className="flex items-center gap-2">
                        <button
                            onClick={handleToggleEnabled}
                            className={clsx(
                                "px-4 py-2 text-sm font-medium rounded-lg transition-colors",
                                extension.enabled
                                    ? "text-amber-600 bg-amber-50 hover:bg-amber-100 dark:bg-amber-900/20 dark:hover:bg-amber-900/30"
                                    : "text-green-600 bg-green-50 hover:bg-green-100 dark:bg-green-900/20 dark:hover:bg-green-900/30"
                            )}
                        >
                            {extension.enabled ? (
                                <>
                                    <Pause className="h-4 w-4 inline mr-1" />
                                    Disable
                                </>
                            ) : (
                                <>
                                    <Play className="h-4 w-4 inline mr-1" />
                                    Enable
                                </>
                            )}
                        </button>
                        <button
                            onClick={handleUninstall}
                            className="px-4 py-2 text-sm font-medium text-red-600 bg-red-50 hover:bg-red-100 dark:bg-red-900/20 dark:hover:bg-red-900/30 rounded-lg transition-colors"
                        >
                            <Trash2 className="h-4 w-4 inline mr-1" />
                            Uninstall
                        </button>
                    </div>
                </div>
            </div>

            {/* Permissions */}
            <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6 mb-6">
                <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Permissions</h2>
                <div className="flex flex-wrap gap-2">
                    {extension.permissions.map((perm) => (
                        <span
                            key={perm}
                            className="px-3 py-1.5 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg text-sm"
                        >
                            {perm}
                        </span>
                    ))}
                </div>
            </div>

            {/* Automation Jobs (only for automation type) */}
            {extension.type === 'automation' && (
                <div className="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 p-6">
                    <div className="flex items-center justify-between mb-4">
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Automation Jobs</h2>
                        <button
                            onClick={() => setShowCreateJobModal(true)}
                            className="inline-flex items-center px-3 py-1.5 text-sm font-medium text-primary-600 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-lg transition-colors"
                        >
                            <Plus className="h-4 w-4 mr-1" />
                            New Job
                        </button>
                    </div>

                    {jobs.length === 0 ? (
                        <div className="text-center py-8 text-gray-500 dark:text-gray-400">
                            <Calendar className="h-8 w-8 mx-auto mb-2" />
                            <p>No automation jobs configured</p>
                        </div>
                    ) : (
                        <div className="space-y-3">
                            {jobs.map((job) => (
                                <div
                                    key={job.id}
                                    className="flex items-center justify-between p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg"
                                >
                                    <div>
                                        <div className="flex items-center gap-2">
                                            <h3 className="font-medium text-gray-900 dark:text-white">{job.name}</h3>
                                            {job.enabled ? (
                                                <span className="px-2 py-0.5 text-xs bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400 rounded">
                                                    Active
                                                </span>
                                            ) : (
                                                <span className="px-2 py-0.5 text-xs bg-gray-100 dark:bg-gray-600 text-gray-600 dark:text-gray-400 rounded">
                                                    Disabled
                                                </span>
                                            )}
                                            {job.last_status && (
                                                <span className={clsx(
                                                    "px-2 py-0.5 text-xs rounded",
                                                    job.last_status === 'success'
                                                        ? "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400"
                                                        : job.last_status === 'failed'
                                                        ? "bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-400"
                                                        : "bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400"
                                                )}>
                                                    Last: {job.last_status}
                                                </span>
                                            )}
                                        </div>
                                        <div className="mt-1 flex items-center gap-4 text-sm text-gray-500 dark:text-gray-400">
                                            <span>Cron: {job.cron_expression}</span>
                                            <span>Next run: {formatDate(job.next_run_at)}</span>
                                            {job.last_run_at && (
                                                <span>Last run: {formatDate(job.last_run_at)}</span>
                                            )}
                                        </div>
                                    </div>
                                    <button
                                        onClick={() => handleTriggerJob(job.id)}
                                        disabled={triggering === job.id}
                                        className="px-3 py-1.5 text-sm font-medium text-primary-600 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-lg transition-colors disabled:opacity-50"
                                    >
                                        {triggering === job.id ? (
                                            <RefreshCw className="h-4 w-4 animate-spin" />
                                        ) : (
                                            <>
                                                <Play className="h-4 w-4 inline mr-1" />
                                                Run Now
                                            </>
                                        )}
                                    </button>
                                </div>
                            ))}
                        </div>
                    )}
                </div>
            )}

            {/* Create Job Modal */}
            {showCreateJobModal && (
                <div className="fixed inset-0 z-50 overflow-y-auto">
                    <div className="flex min-h-full items-center justify-center p-4">
                        <div className="fixed inset-0 bg-black/50" onClick={() => setShowCreateJobModal(false)} />
                        <div className="relative bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full p-6">
                            <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">
                                Create Automation Job
                            </h2>

                            <div className="space-y-4">
                                <div>
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                        Job Name
                                    </label>
                                    <input
                                        type="text"
                                        value={newJobName}
                                        onChange={(e) => setNewJobName(e.target.value)}
                                        placeholder="Daily Backup"
                                        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                                    />
                                </div>

                                <div>
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                        Cron Expression
                                    </label>
                                    <input
                                        type="text"
                                        value={newJobCron}
                                        onChange={(e) => setNewJobCron(e.target.value)}
                                        placeholder="0 0 * * *"
                                        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent font-mono"
                                    />
                                    <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                                        Format: minute hour day month weekday (e.g., "0 0 * * *" for daily at midnight)
                                    </p>
                                </div>
                            </div>

                            <div className="mt-6 flex justify-end gap-3">
                                <button
                                    onClick={() => setShowCreateJobModal(false)}
                                    className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg"
                                >
                                    Cancel
                                </button>
                                <button
                                    onClick={handleCreateJob}
                                    disabled={!newJobName.trim() || !newJobCron.trim()}
                                    className="px-4 py-2 text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 rounded-lg disabled:opacity-50"
                                >
                                    Create Job
                                </button>
                            </div>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}

export default ExtensionDetails;

