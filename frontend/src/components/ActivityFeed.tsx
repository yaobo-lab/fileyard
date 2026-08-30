import { useState, useEffect, useCallback } from 'react';
import { 
    FileText, LogIn, LogOut, Upload, Download, ShieldAlert, Eye, 
    Share2, Lock, Unlock, Trash2, FolderPlus, Move, Edit, 
    UserPlus, UserMinus, UserX, UserCheck, RefreshCw, Settings, 
    Key, Shield, Mail, RotateCcw, FileX, Loader2, AlertCircle
} from 'lucide-react';
import clsx from 'clsx';
import { useAuthFetch, useAuth } from '../context/AuthContext';
import { formatDistanceToNow } from 'date-fns';

interface Activity {
    id: string;
    user: string;
    user_id?: string;
    action: string;
    action_display: string;
    resource: string;
    resource_type: string;
    description: string;
    timestamp: string;
    status: string;
    ip_address?: string;
    metadata?: Record<string, any>;
}

interface ActivityFeedProps {
    limit?: number;
}

export function ActivityFeed({ limit = 10 }: ActivityFeedProps) {
    const [activities, setActivities] = useState<Activity[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [isExporting, setIsExporting] = useState(false);
    const [isRefreshing, setIsRefreshing] = useState(false);
    const [exportError, setExportError] = useState<string | null>(null);
    const authFetch = useAuthFetch();
    const { user } = useAuth();

    const fetchActivities = useCallback(async (showRefreshState = false) => {
        if (showRefreshState) setIsRefreshing(true);
        try {
            const res = await authFetch(`/api/activity-logs?limit=${limit}`);
            if (res.ok) {
                const data = await res.json();
                setActivities(data.logs || []);
            }
        } catch (error) {
            console.error('Failed to fetch activity logs', error);
        } finally {
            setIsLoading(false);
            setIsRefreshing(false);
        }
    }, [authFetch, limit]);

    useEffect(() => {
        fetchActivities();
        const interval = setInterval(() => fetchActivities(false), 15000); // Refresh every 15s
        return () => clearInterval(interval);
    }, [fetchActivities]);

    const handleExport = async () => {
        // Only admins can export
        if (!user || !['Admin', 'SuperAdmin'].includes(user.role)) {
            setExportError('Only admins can export audit logs');
            return;
        }

        setIsExporting(true);
        setExportError(null);
        try {
            const response = await authFetch('/api/activity-logs/export');
            if (response.ok) {
                const blob = await response.blob();
                const url = URL.createObjectURL(blob);
                const a = document.createElement('a');
                a.href = url;
                a.download = `audit-trail-${new Date().toISOString().split('T')[0]}.csv`;
                document.body.appendChild(a);
                a.click();
                document.body.removeChild(a);
                URL.revokeObjectURL(url);
            } else {
                const error = await response.text();
                setExportError(error || 'Failed to export audit trail');
            }
        } catch (error) {
            console.error('Failed to export audit logs', error);
            setExportError('Failed to export audit trail');
        } finally {
            setIsExporting(false);
        }
    };

    const handleRefresh = () => {
        fetchActivities(true);
    };

    const getIcon = (action: string) => {
        // File operations
        if (action === 'file_upload') return <Upload className="w-4 h-4 text-blue-500" />;
        if (action === 'file_download' || action === 'folder_download') return <Download className="w-4 h-4 text-green-500" />;
        if (action === 'file_preview' || action === 'file_view') return <Eye className="w-4 h-4 text-indigo-500" />;
        if (action === 'file_shared' || action === 'shared_download') return <Share2 className="w-4 h-4 text-purple-500" />;
        if (action === 'file_move') return <Move className="w-4 h-4 text-orange-500" />;
        if (action === 'file_rename') return <Edit className="w-4 h-4 text-yellow-500" />;
        if (action === 'file_delete') return <Trash2 className="w-4 h-4 text-red-400" />;
        if (action === 'file_permanent_delete') return <FileX className="w-4 h-4 text-red-600" />;
        if (action === 'file_restore') return <RotateCcw className="w-4 h-4 text-green-600" />;
        if (action === 'file_lock') return <Lock className="w-4 h-4 text-amber-500" />;
        if (action === 'file_unlock') return <Unlock className="w-4 h-4 text-amber-400" />;
        if (action === 'folder_create') return <FolderPlus className="w-4 h-4 text-blue-400" />;
        
        // User operations
        if (action === 'user_created') return <UserPlus className="w-4 h-4 text-green-500" />;
        if (action === 'user_deleted' || action === 'user_permanently_deleted') return <UserMinus className="w-4 h-4 text-red-500" />;
        if (action === 'user_suspended') return <UserX className="w-4 h-4 text-orange-500" />;
        if (action === 'user_activated') return <UserCheck className="w-4 h-4 text-green-500" />;
        if (action === 'user_updated' || action === 'role_change') return <UserCheck className="w-4 h-4 text-blue-500" />;
        
        // Authentication
        if (action === 'login' || action === 'login_success') return <LogIn className="w-4 h-4 text-gray-500" />;
        if (action === 'logout') return <LogOut className="w-4 h-4 text-gray-400" />;
        if (action === 'login_failed') return <ShieldAlert className="w-4 h-4 text-red-500" />;
        if (action === 'session_revoked') return <Key className="w-4 h-4 text-orange-500" />;
        if (action === 'password_changed' || action === 'admin_reset_password') return <Key className="w-4 h-4 text-blue-500" />;
        if (action === 'send_password_reset_email') return <Mail className="w-4 h-4 text-blue-400" />;
        if (action === 'admin_change_email') return <Mail className="w-4 h-4 text-purple-500" />;
        
        // MFA
        if (action === 'mfa_enabled' || action === 'mfa_disabled') return <Shield className="w-4 h-4 text-purple-500" />;
        
        // Security alerts
        if (action.includes('security') || action.includes('alert')) return <ShieldAlert className="w-4 h-4 text-red-500" />;
        
        // Settings
        if (action.includes('settings') || action.includes('compliance') || action.includes('tenant')) {
            return <Settings className="w-4 h-4 text-gray-500" />;
        }
        
        // PHI access (HIPAA)
        if (action === 'phi_access') return <Eye className="w-4 h-4 text-purple-600" />;
        
        // Default
        return <FileText className="w-4 h-4 text-gray-400" />;
    };

    const getIconBackground = (action: string, status: string) => {
        if (status === 'warning' || status === 'error' || action === 'login_failed') {
            return 'bg-red-100 dark:bg-red-900/30';
        }
        if (action === 'phi_access') return 'bg-purple-100 dark:bg-purple-900/30';
        if (action.includes('security') || action.includes('alert')) return 'bg-red-100 dark:bg-red-900/30';
        if (action.includes('delete') || action === 'user_suspended') return 'bg-red-50 dark:bg-red-900/20';
        if (action.includes('upload') || action.includes('create') || action === 'user_created') return 'bg-blue-50 dark:bg-blue-900/20';
        if (action.includes('download') || action === 'file_restore') return 'bg-green-50 dark:bg-green-900/20';
        if (action.includes('share')) return 'bg-purple-50 dark:bg-purple-900/20';
        return 'bg-gray-100 dark:bg-gray-700';
    };

    const canExport = user && ['Admin', 'SuperAdmin'].includes(user.role);

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 transition-colors duration-200 h-full flex flex-col">
            <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex justify-between items-center">
                <div className="flex items-center gap-2">
                    <h3 className="text-lg font-medium text-gray-900 dark:text-white">Activity Log</h3>
                    <button 
                        onClick={handleRefresh}
                        disabled={isRefreshing}
                        className="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
                        title="Refresh"
                    >
                        <RefreshCw className={clsx("w-4 h-4", isRefreshing && "animate-spin")} />
                    </button>
                </div>
                {canExport && (
                    <div className="flex items-center gap-2">
                        {exportError && (
                            <span className="text-xs text-red-500 flex items-center gap-1">
                                <AlertCircle className="w-3 h-3" />
                                {exportError}
                            </span>
                        )}
                        <button 
                            onClick={handleExport}
                            disabled={isExporting}
                            className="text-sm text-primary-600 hover:text-primary-700 dark:text-primary-400 dark:hover:text-primary-300 font-medium flex items-center gap-1 disabled:opacity-50"
                        >
                            {isExporting ? (
                                <>
                                    <Loader2 className="w-3 h-3 animate-spin" />
                                    Exporting...
                                </>
                            ) : (
                                'Export Audit Trail'
                            )}
                        </button>
                    </div>
                )}
            </div>
            <div className="flow-root flex-1 overflow-auto">
                {isLoading ? (
                    <div className="flex items-center justify-center py-12">
                        <Loader2 className="w-6 h-6 text-gray-400 animate-spin" />
                    </div>
                ) : (
                    <ul className="divide-y divide-gray-200 dark:divide-gray-700">
                        {activities.map((activity) => (
                            <li key={activity.id} className="px-6 py-4 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                                <div className="flex items-center space-x-4">
                                    <div className={clsx(
                                        "flex-shrink-0 h-8 w-8 rounded-full flex items-center justify-center",
                                        getIconBackground(activity.action, activity.status)
                                    )}>
                                        {getIcon(activity.action)}
                                    </div>
                                    <div className="flex-1 min-w-0">
                                        <p className="text-sm font-medium text-gray-900 dark:text-white truncate">
                                            {activity.user}
                                        </p>
                                        <p className="text-sm text-gray-500 dark:text-gray-400 truncate">
                                            {activity.action_display || activity.description || (
                                                <>
                                                    {activity.action.replace(/_/g, ' ')} <span className="font-medium text-gray-700 dark:text-gray-300">{activity.resource}</span>
                                                </>
                                            )}
                                        </p>
                                    </div>
                                    <div className="inline-flex items-center text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap">
                                        {formatDistanceToNow(new Date(activity.timestamp), { addSuffix: true })}
                                    </div>
                                </div>
                            </li>
                        ))}
                        {activities.length === 0 && (
                            <li className="px-6 py-8 text-center text-gray-500 dark:text-gray-400">
                                No recent activity found.
                            </li>
                        )}
                    </ul>
                )}
            </div>
        </div>
    );
}
