import { useState, useEffect } from 'react';
import { Clock, FileText, Upload } from 'lucide-react';
import { useAuthFetch } from '../../context/AuthContext';
import { formatDistanceToNow } from 'date-fns';

interface RecentFile {
    id: string;
    name: string;
    size_bytes: number;
    created_at: string;
    content_type: string;
}

interface RecentUploadsWidgetProps {
    limit?: number;
}

export function RecentUploadsWidget({ limit = 5 }: RecentUploadsWidgetProps) {
    const [files, setFiles] = useState<RecentFile[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const authFetch = useAuthFetch();

    useEffect(() => {
        fetchRecentUploads();
    }, [limit]);

    const fetchRecentUploads = async () => {
        try {
            const res = await authFetch('/api/activity-logs?action=upload&limit=' + limit);
            if (res.ok) {
                const data = await res.json();
                // Extract file info from activity logs
                const recentFiles = (data.logs || []).map((log: any) => ({
                    id: log.id,
                    name: log.resource || 'Unknown file',
                    size_bytes: log.metadata?.size_bytes || 0,
                    created_at: log.timestamp,
                    content_type: log.metadata?.content_type || 'application/octet-stream'
                }));
                setFiles(recentFiles);
            }
        } catch (error) {
            console.error('Failed to fetch recent uploads', error);
        } finally {
            setIsLoading(false);
        }
    };

    const formatFileSize = (bytes: number) => {
        if (bytes === 0) return '';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
    };

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-6 h-full">
            <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-medium text-gray-900 dark:text-white">Recent Uploads</h3>
                <Upload className="w-5 h-5 text-gray-400" />
            </div>
            
            {isLoading ? (
                <div className="animate-pulse space-y-3">
                    {[...Array(3)].map((_, i) => (
                        <div key={i} className="h-12 bg-gray-200 dark:bg-gray-700 rounded"></div>
                    ))}
                </div>
            ) : files.length === 0 ? (
                <div className="text-center py-6 text-gray-500 dark:text-gray-400">
                    <Upload className="w-8 h-8 mx-auto mb-2 opacity-50" />
                    <p className="text-sm">No recent uploads</p>
                </div>
            ) : (
                <div className="space-y-3">
                    {files.map((file) => (
                        <div key={file.id} className="flex items-center p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                            <div className="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg mr-3">
                                <FileText className="w-4 h-4 text-blue-600 dark:text-blue-400" />
                            </div>
                            <div className="flex-1 min-w-0">
                                <p className="text-sm font-medium text-gray-900 dark:text-white truncate">{file.name}</p>
                                <p className="text-xs text-gray-500 dark:text-gray-400">
                                    {formatFileSize(file.size_bytes)}
                                    {file.size_bytes > 0 && ' • '}
                                    {formatDistanceToNow(new Date(file.created_at), { addSuffix: true })}
                                </p>
                            </div>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
