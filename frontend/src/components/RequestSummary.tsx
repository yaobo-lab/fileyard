import { useState, useEffect } from 'react';
import { Link as LinkIcon, Clock, ArrowRight, Upload } from 'lucide-react';
import { Link } from 'react-router-dom';
import { useAuthFetch } from '../context/AuthContext';

interface FileRequest {
    id: string;
    name: string;
    upload_count: number;
    expires_at: string;
    expiry_text: string;
    has_new_uploads: boolean;
}

interface DashboardStats {
    file_requests: FileRequest[];
    total_active_requests: number;
}

export function RequestSummary() {
    const [requests, setRequests] = useState<FileRequest[]>([]);
    const [totalActive, setTotalActive] = useState(0);
    const [isLoading, setIsLoading] = useState(true);
    const authFetch = useAuthFetch();

    useEffect(() => {
        fetchRequests();
    }, []);

    const fetchRequests = async () => {
        try {
            const res = await authFetch('/api/dashboard/stats');
            if (res.ok) {
                const data: DashboardStats = await res.json();
                setRequests(data.file_requests || []);
                setTotalActive(data.total_active_requests || 0);
            }
        } catch (error) {
            console.error('Failed to fetch file requests', error);
        } finally {
            setIsLoading(false);
        }
    };

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-6 transition-colors duration-200 h-full">
            <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-medium text-gray-900 dark:text-white">Active File Requests</h3>
                <Link to="/file-requests" className="text-sm text-primary-600 hover:text-primary-700 dark:text-primary-400 dark:hover:text-primary-300 font-medium flex items-center">
                    Manage <ArrowRight className="w-4 h-4 ml-1" />
                </Link>
            </div>

            <div className="space-y-4">
                {isLoading ? (
                    <div className="animate-pulse space-y-4">
                        <div className="h-16 bg-gray-200 dark:bg-gray-700 rounded-md"></div>
                        <div className="h-16 bg-gray-200 dark:bg-gray-700 rounded-md"></div>
                    </div>
                ) : requests.length === 0 ? (
                    <div className="text-center py-6 text-gray-500 dark:text-gray-400">
                        <LinkIcon className="w-8 h-8 mx-auto mb-2 opacity-50" />
                        <p className="text-sm">No active file requests</p>
                        <Link to="/file-requests" className="text-sm text-primary-600 hover:text-primary-700 dark:text-primary-400 mt-2 inline-block">
                            Create one
                        </Link>
                    </div>
                ) : (
                    requests.map((request) => (
                        <div 
                            key={request.id}
                            className={`flex items-center justify-between p-3 rounded-md border ${
                                request.has_new_uploads 
                                    ? 'bg-blue-50 dark:bg-blue-900/20 border-blue-100 dark:border-blue-800'
                                    : 'bg-gray-50 dark:bg-gray-700/50 border-gray-200 dark:border-gray-600'
                            }`}
                        >
                            <div className="flex items-center">
                                <div className={`h-8 w-8 rounded-full flex items-center justify-center mr-3 ${
                                    request.has_new_uploads 
                                        ? 'bg-blue-100 dark:bg-blue-900/50 text-blue-600 dark:text-blue-400'
                                        : 'bg-gray-200 dark:bg-gray-600 text-gray-600 dark:text-gray-300'
                                }`}>
                                    <LinkIcon className="w-4 h-4" />
                                </div>
                                <div>
                                    <p className="text-sm font-medium text-gray-900 dark:text-white">{request.name}</p>
                                    <p className={`text-xs ${
                                        request.has_new_uploads 
                                            ? 'text-blue-600 dark:text-blue-400'
                                            : 'text-gray-500 dark:text-gray-400'
                                    }`}>
                                        {request.upload_count > 0 ? (
                                            <span className="flex items-center">
                                                <Upload className="w-3 h-3 mr-1" />
                                                {request.upload_count} upload{request.upload_count !== 1 ? 's' : ''}
                                            </span>
                                        ) : (
                                            'No uploads yet'
                                        )}
                                    </p>
                                </div>
                            </div>
                            <div className="text-xs text-gray-500 dark:text-gray-400 flex items-center">
                                <Clock className="w-3 h-3 mr-1" />
                                {request.expiry_text}
                            </div>
                        </div>
                    ))
                )}
            </div>

            <div className="mt-4 pt-4 border-t border-gray-100 dark:border-gray-700">
                <div className="flex justify-between text-sm">
                    <span className="text-gray-500 dark:text-gray-400">Total Active Links</span>
                    <span className="font-medium text-gray-900 dark:text-white">{totalActive}</span>
                </div>
            </div>
        </div>
    );
}
