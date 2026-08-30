import { useState, useEffect } from 'react';
import { X, Link2, Calendar, FolderOpen, FileText, Download, Clock, User } from 'lucide-react';
import { useAuthFetch } from '../context/AuthContext';
import { useGlobalSettings } from '../context/GlobalSettingsContext';
import clsx from 'clsx';

interface FileRequest {
    id: string;
    name: string;
    destination: string;
    created_at: string;
    expires_at: string;
    upload_count: number;
    status: 'active' | 'expired' | 'revoked';
    link: string;
    max_uploads?: number;
}

interface Upload {
    id: string;
    filename: string;
    size: number;
    uploaded_at: string;
    uploader_name?: string;
    uploader_email?: string;
}

interface FileRequestDetailsModalProps {
    isOpen: boolean;
    onClose: () => void;
    request: FileRequest | null;
}

export function FileRequestDetailsModal({ isOpen, onClose, request }: FileRequestDetailsModalProps) {
    const [uploads, setUploads] = useState<Upload[]>([]);
    const [isLoading, setIsLoading] = useState(false);
    const authFetch = useAuthFetch();
    const { formatDate: globalFormatDate } = useGlobalSettings();

    useEffect(() => {
        if (isOpen && request) {
            fetchUploads();
        }
    }, [isOpen, request]);

    const fetchUploads = async () => {
        if (!request) return;
        
        setIsLoading(true);
        try {
            const response = await authFetch(`/api/file-requests/${request.id}/uploads`);
            if (response.ok) {
                const data = await response.json();
                setUploads(data || []);
            }
        } catch (error) {
            console.error('Failed to fetch uploads:', error);
        } finally {
            setIsLoading(false);
        }
    };

    const formatDate = (dateString: string) => {
        return new Date(dateString).toLocaleDateString('en-US', {
            month: 'short',
            day: 'numeric',
            year: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
        });
    };

    const formatFileSize = (bytes: number) => {
        if (bytes < 1024) return `${bytes} B`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
        return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
    };

    const getStatusColor = (status: string) => {
        switch (status) {
            case 'active':
                return 'bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300';
            case 'expired':
                return 'bg-yellow-100 dark:bg-yellow-900/30 text-yellow-800 dark:text-yellow-300';
            default:
                return 'bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-300';
        }
    };

    if (!isOpen || !request) return null;

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center overflow-y-auto">
            <div className="fixed inset-0 bg-black/50 backdrop-blur-sm" onClick={onClose} />
            
            <div className="relative w-full max-w-2xl mx-4 bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-h-[90vh] flex flex-col">
                {/* Header */}
                <div className="p-6 border-b border-gray-200 dark:border-gray-700 flex items-start justify-between">
                    <div className="flex items-start gap-4">
                        <div className="p-3 rounded-xl bg-primary-100 dark:bg-primary-900/30">
                            <Link2 className="w-6 h-6 text-primary-600 dark:text-primary-400" />
                        </div>
                        <div>
                            <h2 className="text-xl font-semibold text-gray-900 dark:text-white">{request.name}</h2>
                            <p className="text-sm text-gray-500 dark:text-gray-400 mt-1 flex items-center gap-2">
                                <FolderOpen className="w-4 h-4" />
                                {request.destination}
                            </p>
                        </div>
                    </div>
                    <button
                        onClick={onClose}
                        className="p-2 rounded-full hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <X className="w-5 h-5 text-gray-500" />
                    </button>
                </div>

                {/* Request Info */}
                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                    <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
                        <div>
                            <p className="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider">Status</p>
                            <span className={clsx(
                                "inline-flex items-center mt-1 px-2 py-0.5 rounded-full text-xs font-medium",
                                getStatusColor(request.status)
                            )}>
                                {request.status.charAt(0).toUpperCase() + request.status.slice(1)}
                            </span>
                        </div>
                        <div>
                            <p className="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider">Uploads</p>
                            <p className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                                {request.upload_count} {request.max_uploads ? `/ ${request.max_uploads}` : ''} files
                            </p>
                        </div>
                        <div>
                            <p className="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider">Created</p>
                            <p className="mt-1 text-sm text-gray-900 dark:text-white flex items-center gap-1">
                                <Calendar className="w-3 h-3 text-gray-400" />
                                {globalFormatDate(request.created_at)}
                            </p>
                        </div>
                        <div>
                            <p className="text-xs text-gray-500 dark:text-gray-400 uppercase tracking-wider">Expires</p>
                            <p className="mt-1 text-sm text-gray-900 dark:text-white flex items-center gap-1">
                                <Clock className="w-3 h-3 text-gray-400" />
                                {globalFormatDate(request.expires_at)}
                            </p>
                        </div>
                    </div>
                </div>

                {/* Uploads List */}
                <div className="flex-1 overflow-y-auto p-6">
                    <h3 className="text-sm font-semibold text-gray-900 dark:text-white mb-4">
                        Uploaded Files ({uploads.length})
                    </h3>
                    
                    {isLoading ? (
                        <div className="flex items-center justify-center py-8">
                            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
                        </div>
                    ) : uploads.length === 0 ? (
                        <div className="text-center py-8 text-gray-500 dark:text-gray-400">
                            <FileText className="w-12 h-12 mx-auto mb-4 opacity-50" />
                            <p>No files uploaded yet</p>
                        </div>
                    ) : (
                        <div className="space-y-3">
                            {uploads.map((upload) => (
                                <div
                                    key={upload.id}
                                    className="flex items-center gap-4 p-4 rounded-lg bg-gray-50 dark:bg-gray-700/50 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                                >
                                    <div className="p-2 rounded-lg bg-white dark:bg-gray-600">
                                        <FileText className="w-5 h-5 text-gray-500 dark:text-gray-400" />
                                    </div>
                                    <div className="flex-1 min-w-0">
                                        <p className="text-sm font-medium text-gray-900 dark:text-white truncate">
                                            {upload.filename}
                                        </p>
                                        <div className="flex items-center gap-3 mt-1 text-xs text-gray-500 dark:text-gray-400">
                                            <span>{formatFileSize(upload.size)}</span>
                                            <span>•</span>
                                            <span>{formatDate(upload.uploaded_at)}</span>
                                            {upload.uploader_name && (
                                                <>
                                                    <span>•</span>
                                                    <span className="flex items-center gap-1">
                                                        <User className="w-3 h-3" />
                                                        {upload.uploader_name}
                                                    </span>
                                                </>
                                            )}
                                        </div>
                                    </div>
                                    <button
                                        className="p-2 text-gray-400 hover:text-primary-600 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-lg transition-colors"
                                        title="Download"
                                    >
                                        <Download className="w-4 h-4" />
                                    </button>
                                </div>
                            ))}
                        </div>
                    )}
                </div>

                {/* Footer */}
                <div className="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex items-center justify-between">
                    <div className="text-xs text-gray-500 dark:text-gray-400">
                        Link: <code className="bg-gray-100 dark:bg-gray-700 px-1 py-0.5 rounded">{request.link}</code>
                    </div>
                    <button
                        onClick={onClose}
                        className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                    >
                        Close
                    </button>
                </div>
            </div>
        </div>
    );
}

