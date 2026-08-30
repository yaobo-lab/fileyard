import { useState, useEffect } from 'react';
import { X, Clock, User, FileText, Download, Upload, Edit2, Trash2, Lock, Unlock, Eye, Share2, FolderDown, FolderPlus, RotateCcw, Link, Move } from 'lucide-react';
import { useAuthFetch } from '../context/AuthContext';
import { useTenant } from '../context/TenantContext';
import { formatDistanceToNow } from 'date-fns';

interface Activity {
    id: string;
    action: string;
    user_id: string | null;
    user_name: string;
    metadata: any;
    created_at: string;
}

interface FileActivityModalProps {
    isOpen: boolean;
    onClose: () => void;
    fileId: string;
    fileName: string;
}

const getActionIcon = (action: string) => {
    switch (action) {
        case 'file_upload':
            return <Upload className="w-4 h-4 text-green-500" />;
        case 'file_download':
            return <Download className="w-4 h-4 text-blue-500" />;
        case 'file_preview':
            return <Eye className="w-4 h-4 text-purple-500" />;
        case 'file_view':
            return <Eye className="w-4 h-4 text-purple-500" />;
        case 'file_rename':
            return <Edit2 className="w-4 h-4 text-yellow-500" />;
        case 'file_delete':
            return <Trash2 className="w-4 h-4 text-red-500" />;
        case 'file_move':
            return <Move className="w-4 h-4 text-blue-500" />;
        case 'file_lock':
            return <Lock className="w-4 h-4 text-orange-500" />;
        case 'file_unlock':
            return <Unlock className="w-4 h-4 text-green-500" />;
        case 'file_shared':
            return <Share2 className="w-4 h-4 text-indigo-500" />;
        case 'file_restore':
            return <RotateCcw className="w-4 h-4 text-green-500" />;
        case 'file_permanent_delete':
            return <Trash2 className="w-4 h-4 text-red-600" />;
        case 'folder_download':
            return <FolderDown className="w-4 h-4 text-blue-500" />;
        case 'folder_create':
            return <FolderPlus className="w-4 h-4 text-green-500" />;
        case 'shared_download':
            return <Link className="w-4 h-4 text-cyan-500" />;
        case 'file_export':
            return <Download className="w-4 h-4 text-teal-500" />;
        default:
            return <FileText className="w-4 h-4 text-gray-500" />;
    }
};

const getActionLabel = (action: string) => {
    switch (action) {
        case 'file_upload':
            return 'Uploaded file';
        case 'file_download':
            return 'Downloaded file';
        case 'file_preview':
            return 'Previewed file';
        case 'file_view':
            return 'Viewed file';
        case 'file_rename':
            return 'Renamed file';
        case 'file_delete':
            return 'Deleted file';
        case 'file_move':
            return 'Moved file';
        case 'file_lock':
            return 'Locked file';
        case 'file_unlock':
            return 'Unlocked file';
        case 'file_shared':
            return 'Shared file';
        case 'file_restore':
            return 'Restored file';
        case 'file_permanent_delete':
            return 'Permanently deleted file';
        case 'folder_download':
            return 'Downloaded folder';
        case 'folder_create':
            return 'Created folder';
        case 'shared_download':
            return 'Downloaded via share link';
        case 'file_export':
            return 'Exported file';
        default:
            // Fallback: capitalize words and replace underscores
            return action.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase());
    }
};

export function FileActivityModal({ isOpen, onClose, fileId, fileName }: FileActivityModalProps) {
    const [activities, setActivities] = useState<Activity[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const authFetch = useAuthFetch();
    const { currentCompany } = useTenant();

    useEffect(() => {
        if (isOpen && fileId) {
            fetchActivity();
        }
    }, [isOpen, fileId]);

    const fetchActivity = async () => {
        setIsLoading(true);
        try {
            const response = await authFetch(`/api/files/${currentCompany.id}/${fileId}/activity?limit=50`);
            if (response.ok) {
                const data = await response.json();
                setActivities(data.activities || []);
            }
        } catch (error) {
            console.error('Failed to fetch file activity:', error);
        } finally {
            setIsLoading(false);
        }
    };

    const downloadCSV = () => {
        if (activities.length === 0) return;
        
        // Escape CSV field values
        const escapeCSV = (value: string) => {
            if (value.includes(',') || value.includes('"') || value.includes('\n')) {
                return `"${value.replace(/"/g, '""')}"`;
            }
            return value;
        };
        
        const headers = ['Action', 'User', 'Date', 'Details'];
        const rows = activities.map(a => [
            escapeCSV(getActionLabel(a.action)),
            escapeCSV(a.user_name || 'Unknown'),
            escapeCSV(new Date(a.created_at).toLocaleString()),
            escapeCSV(a.metadata ? JSON.stringify(a.metadata) : '')
        ]);
        
        const csv = [
            headers.join(','),
            ...rows.map(row => row.join(','))
        ].join('\n');
        
        const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' });
        const url = URL.createObjectURL(blob);
        const link = document.createElement('a');
        link.href = url;
        link.download = `${fileName.replace(/[^a-z0-9]/gi, '_')}_activity_${new Date().toISOString().split('T')[0]}.csv`;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        URL.revokeObjectURL(url);
    };

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 z-50 overflow-y-auto">
            <div className="flex min-h-full items-center justify-center p-4">
                {/* Backdrop */}
                <div 
                    className="fixed inset-0 bg-black/50 transition-opacity" 
                    onClick={onClose}
                />

                {/* Modal */}
                <div className="relative bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-lg transform transition-all">
                    {/* Header */}
                    <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                        <div>
                            <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                                Recent Activity
                            </h2>
                            <p className="text-sm text-gray-500 dark:text-gray-400 mt-0.5 truncate max-w-[300px]">
                                {fileName}
                            </p>
                        </div>
                        <button
                            onClick={onClose}
                            className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                        >
                            <X className="w-5 h-5" />
                        </button>
                    </div>

                    {/* Content */}
                    <div className="px-6 py-4 max-h-[400px] overflow-y-auto">
                        {isLoading ? (
                            <div className="flex items-center justify-center py-8">
                                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
                            </div>
                        ) : activities.length === 0 ? (
                            <div className="text-center py-8">
                                <Clock className="w-12 h-12 text-gray-300 dark:text-gray-600 mx-auto mb-3" />
                                <p className="text-gray-500 dark:text-gray-400">No activity recorded yet</p>
                                <p className="text-sm text-gray-400 dark:text-gray-500 mt-1">
                                    Activity will appear here when actions are performed on this file
                                </p>
                            </div>
                        ) : (
                            <div className="space-y-4">
                                {activities.map((activity) => (
                                    <div 
                                        key={activity.id}
                                        className="flex items-start space-x-3 p-3 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
                                    >
                                        <div className="flex-shrink-0 p-2 bg-gray-100 dark:bg-gray-700 rounded-lg">
                                            {getActionIcon(activity.action)}
                                        </div>
                                        <div className="flex-1 min-w-0">
                                            <p className="text-sm font-medium text-gray-900 dark:text-white">
                                                {getActionLabel(activity.action)}
                                            </p>
                                            <div className="flex items-center mt-1 text-xs text-gray-500 dark:text-gray-400">
                                                <User className="w-3 h-3 mr-1" />
                                                <span>{activity.user_name}</span>
                                                <span className="mx-2">•</span>
                                                <Clock className="w-3 h-3 mr-1" />
                                                <span>
                                                    {formatDistanceToNow(new Date(activity.created_at), { addSuffix: true })}
                                                </span>
                                            </div>
                                            {activity.metadata && (
                                                <div className="mt-2 text-xs text-gray-400 dark:text-gray-500">
                                                    {activity.metadata.old_name && activity.metadata.new_name && (
                                                        <span>
                                                            Renamed from "{activity.metadata.old_name}" to "{activity.metadata.new_name}"
                                                        </span>
                                                    )}
                                                    {activity.metadata.compliance_mode && (
                                                        <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-300 ml-2">
                                                            {activity.metadata.compliance_mode}
                                                        </span>
                                                    )}
                                                </div>
                                            )}
                                        </div>
                                    </div>
                                ))}
                            </div>
                        )}
                    </div>

                    {/* Footer */}
                    <div className="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-between">
                        <button
                            onClick={downloadCSV}
                            disabled={activities.length === 0 || isLoading}
                            className="flex items-center px-4 py-2 text-sm font-medium text-primary-700 dark:text-primary-300 bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 rounded-lg hover:bg-primary-100 dark:hover:bg-primary-900/30 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            <Download className="w-4 h-4 mr-2" />
                            Export CSV
                        </button>
                        <button
                            onClick={onClose}
                            className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-200 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-600 transition-colors"
                        >
                            Close
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}
