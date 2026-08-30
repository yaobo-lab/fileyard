import { useState, useEffect, useRef } from 'react';
import { Search, Filter, Link as LinkIcon, Calendar, Trash2, Eye, Copy, Check, Plus, Users, EyeOff, ChevronDown } from 'lucide-react';
import clsx from 'clsx';
import { useAuthFetch } from '../context/AuthContext';
import { useGlobalSettings } from '../context/GlobalSettingsContext';
import { FilterModal } from '../components/FilterModal';
import { CreateFileRequestModal, FileRequestData } from '../components/CreateFileRequestModal';
import { FileRequestDetailsModal } from '../components/FileRequestDetailsModal';

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
    visibility?: 'department' | 'private';
}

const statusFilterOptions = [
    { label: 'Active', value: 'active' },
    { label: 'Expired', value: 'expired' },
    { label: 'Revoked', value: 'revoked' },
];

export function FileRequests() {
    const [requests, setRequests] = useState<FileRequest[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [searchTerm, setSearchTerm] = useState('');
    const [copiedId, setCopiedId] = useState<string | null>(null);
    const [isFilterOpen, setIsFilterOpen] = useState(false);
    const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
    const [isDetailsModalOpen, setIsDetailsModalOpen] = useState(false);
    const [selectedRequest, setSelectedRequest] = useState<FileRequest | null>(null);
    const [filters, setFilters] = useState<any>({});
    const authFetch = useAuthFetch();
    const { formatDate } = useGlobalSettings();

    // Visibility mode: 'department' or 'private'
    const [fileViewMode, setFileViewMode] = useState<'department' | 'private'>('department');
    const [isViewModeOpen, setIsViewModeOpen] = useState(false);
    const viewModeRef = useRef<HTMLDivElement>(null);

    // Close view mode dropdown when clicking outside
    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (viewModeRef.current && !viewModeRef.current.contains(event.target as Node)) {
                setIsViewModeOpen(false);
            }
        };
        document.addEventListener('click', handleClickOutside);
        return () => document.removeEventListener('click', handleClickOutside);
    }, []);

    useEffect(() => {
        fetchFileRequests();
    }, [filters, fileViewMode]);

    const fetchFileRequests = async () => {
        try {
            setIsLoading(true);

            // Build query params
            const params = new URLSearchParams();
            params.append('visibility', fileViewMode);
            if (filters.status) params.append('status', filters.status);
            if (filters.dateFrom) params.append('created_after', filters.dateFrom);
            if (filters.dateTo) params.append('created_before', filters.dateTo);

            const response = await authFetch(`/api/file-requests?${params.toString()}`);

            if (!response.ok) {
                throw new Error('Failed to fetch file requests');
            }

            const data = await response.json();
            setRequests(data);
        } catch (error) {
            console.error('Error fetching file requests:', error);
        } finally {
            setIsLoading(false);
        }
    };

    const handleCopy = (link: string, id: string) => {
        navigator.clipboard.writeText(link);
        setCopiedId(id);
        setTimeout(() => setCopiedId(null), 2000);
    };

    const handleDelete = async (id: string) => {
        if (!confirm('Are you sure you want to revoke this file request?')) return;

        try {
            const response = await authFetch(`/api/file-requests/${id}`, {
                method: 'DELETE',
            });

            if (response.ok) {
                fetchFileRequests();
            }
        } catch (error) {
            console.error('Error deleting file request:', error);
        }
    };

    const handlePermanentDelete = async (id: string) => {
        if (!confirm('Are you sure you want to PERMANENTLY DELETE this file request? This cannot be undone.')) return;

        try {
            const response = await authFetch(`/api/file-requests/${id}/permanent`, {
                method: 'DELETE',
            });

            if (response.ok) {
                fetchFileRequests();
            } else {
                const error = await response.json();
                alert(error.error || 'Failed to delete file request');
            }
        } catch (error) {
            console.error('Error permanently deleting file request:', error);
        }
    };

    const handleCreate = async (data: FileRequestData) => {
        const response = await authFetch('/api/file-requests', {
            method: 'POST',
            body: JSON.stringify(data),
        });

        if (!response.ok) {
            throw new Error('Failed to create file request');
        }

        fetchFileRequests();
    };

    const filteredRequests = requests.filter(req =>
        req.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        req.destination.toLowerCase().includes(searchTerm.toLowerCase())
    );

    return (
        <div className="space-y-4 sm:space-y-6">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 sm:gap-4">
                <div>
                    <h1 className="text-xl sm:text-2xl font-bold text-gray-900 dark:text-white">File Requests</h1>
                    <p className="text-xs sm:text-sm text-gray-500 dark:text-gray-400 mt-0.5 sm:mt-1">Manage active upload links and view submission history.</p>
                </div>
                <div className="flex items-center space-x-2 sm:space-x-3">
                    {/* View Mode Switcher */}
                    <div className="relative" ref={viewModeRef}>
                        <button
                            onClick={(e) => { e.stopPropagation(); setIsViewModeOpen(!isViewModeOpen); }}
                            className="flex items-center px-3 sm:px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm font-medium text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700 shadow-sm"
                        >
                            {fileViewMode === 'department' ? (
                                <><Users className="w-4 h-4 sm:mr-2 text-blue-500" /><span className="hidden sm:inline">Department Requests</span></>
                            ) : (
                                <><EyeOff className="w-4 h-4 sm:mr-2 text-purple-500" /><span className="hidden sm:inline">My Private Requests</span></>
                            )}
                            <ChevronDown className="w-4 h-4 ml-1 sm:ml-2 text-gray-400" />
                        </button>
                        {isViewModeOpen && (
                            <div className="absolute right-0 mt-2 w-56 bg-white dark:bg-gray-800 rounded-lg shadow-lg py-1 ring-1 ring-black ring-opacity-5 z-50 border border-gray-200 dark:border-gray-700">
                                <button
                                    onClick={() => { setFileViewMode('department'); setIsViewModeOpen(false); }}
                                    className={clsx(
                                        "flex items-center w-full px-4 py-2.5 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700",
                                        fileViewMode === 'department' && "bg-gray-50 dark:bg-gray-700 font-medium"
                                    )}
                                >
                                    <Users className="w-4 h-4 mr-3 text-blue-500" />
                                    Department Requests
                                    {fileViewMode === 'department' && <span className="ml-auto text-primary-500">✓</span>}
                                </button>
                                <button
                                    onClick={() => { setFileViewMode('private'); setIsViewModeOpen(false); }}
                                    className={clsx(
                                        "flex items-center w-full px-4 py-2.5 text-sm text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700",
                                        fileViewMode === 'private' && "bg-gray-50 dark:bg-gray-700 font-medium"
                                    )}
                                >
                                    <EyeOff className="w-4 h-4 mr-3 text-purple-500" />
                                    My Private Requests
                                    {fileViewMode === 'private' && <span className="ml-auto text-primary-500">✓</span>}
                                </button>
                            </div>
                        )}
                    </div>
                    <button
                        onClick={() => setIsCreateModalOpen(true)}
                        className="px-3 sm:px-4 py-2 bg-primary-600 text-white rounded-lg text-sm font-medium hover:bg-primary-700 shadow-sm flex items-center transition-colors">
                        <Plus className="w-4 h-4 sm:mr-2" />
                        <span className="hidden sm:inline">New Request</span>
                    </button>
                </div>
            </div>

            <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-sm overflow-hidden transition-colors">
                {/* Toolbar */}
                <div className="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between bg-gray-50 dark:bg-gray-900/50">
                    <div className="relative max-w-md w-full">
                        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                            <Search className="h-4 w-4 text-gray-400 dark:text-gray-500" />
                        </div>
                        <input
                            type="text"
                            className="block w-full pl-10 pr-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md leading-5 bg-white dark:bg-gray-700 placeholder-gray-500 dark:placeholder-gray-400 text-gray-900 dark:text-white focus:outline-none focus:placeholder-gray-400 focus:ring-1 focus:ring-primary-500 focus:border-primary-500 sm:text-sm transition-colors"
                            placeholder="Search requests..."
                            value={searchTerm}
                            onChange={(e) => setSearchTerm(e.target.value)}
                        />
                    </div>
                    <button
                        onClick={() => setIsFilterOpen(true)}
                        className="px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md text-sm font-medium text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-600 flex items-center transition-colors"
                    >
                        <Filter className="w-4 h-4 mr-2 text-gray-500 dark:text-gray-400" />
                        Filters
                        {(filters.status || filters.dateFrom || filters.dateTo) && (
                            <span className="ml-2 w-2 h-2 bg-primary-500 rounded-full"></span>
                        )}
                    </button>
                </div>

                {/* Table */}
                <div className="overflow-x-auto">
                    {isLoading ? (
                        <div className="p-8 text-center text-gray-500 dark:text-gray-400">Loading...</div>
                    ) : filteredRequests.length === 0 ? (
                        <div className="p-8 text-center text-gray-500 dark:text-gray-400">No file requests found</div>
                    ) : (
                        <>
                            {/* Mobile: Card view */}
                            <div className="sm:hidden divide-y divide-gray-200 dark:divide-gray-700">
                                {filteredRequests.map((req) => (
                                    <div 
                                        key={req.id} 
                                        className="p-4 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
                                        onClick={() => {
                                            setSelectedRequest(req);
                                            setIsDetailsModalOpen(true);
                                        }}
                                    >
                                        <div className="flex items-start justify-between">
                                            <div className="flex items-center min-w-0 flex-1">
                                                <div className="h-10 w-10 flex-shrink-0 rounded-lg bg-primary-100 dark:bg-primary-900/20 flex items-center justify-center text-primary-700 dark:text-primary-400">
                                                    <LinkIcon className="w-5 h-5" />
                                                </div>
                                                <div className="ml-3 min-w-0 flex-1">
                                                    <p className="text-sm font-medium text-gray-900 dark:text-white truncate">{req.name}</p>
                                                    <p className="text-xs text-gray-500 dark:text-gray-400 truncate">{req.destination}</p>
                                                    <div className="flex items-center gap-2 mt-1.5 flex-wrap">
                                                        <span className={clsx(
                                                            "px-2 py-0.5 text-xs font-semibold rounded-full",
                                                            req.status === 'active' ? "bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300" :
                                                                req.status === 'expired' ? "bg-yellow-100 dark:bg-yellow-900/30 text-yellow-800 dark:text-yellow-300" :
                                                                    "bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-300"
                                                        )}>
                                                            {req.status.charAt(0).toUpperCase() + req.status.slice(1)}
                                                        </span>
                                                        <span className="text-xs text-gray-500 dark:text-gray-400">
                                                            {req.upload_count} files
                                                        </span>
                                                    </div>
                                                </div>
                                            </div>
                                            <div className="flex items-center gap-2 ml-2" onClick={e => e.stopPropagation()}>
                                                <button
                                                    onClick={() => handleCopy(req.link, req.id)}
                                                    className="p-2 text-gray-400 hover:text-primary-600 dark:hover:text-primary-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg"
                                                    title="Copy Link"
                                                >
                                                    {copiedId === req.id ? <Check className="w-5 h-5 text-green-500" /> : <Copy className="w-5 h-5" />}
                                                </button>
                                                {req.status === 'active' ? (
                                                    <button
                                                        onClick={() => handleDelete(req.id)}
                                                        className="p-2 text-gray-400 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg"
                                                        title="Revoke"
                                                    >
                                                        <Trash2 className="w-5 h-5" />
                                                    </button>
                                                ) : (
                                                    <button
                                                        onClick={() => handlePermanentDelete(req.id)}
                                                        className="p-2 text-gray-400 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg"
                                                        title="Delete"
                                                    >
                                                        <Trash2 className="w-5 h-5" />
                                                    </button>
                                                )}
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                            
                            {/* Desktop: Table view */}
                            <table className="hidden sm:table min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                <thead className="bg-gray-50 dark:bg-gray-700/50">
                                    <tr>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Request Name</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Destination</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Uploads</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Status</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Expires</th>
                                        <th className="relative px-6 py-3"><span className="sr-only">Actions</span></th>
                                    </tr>
                                </thead>
                                <tbody className="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                                    {filteredRequests.map((req) => (
                                        <tr key={req.id} className="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                                            <td className="px-6 py-4 whitespace-nowrap">
                                                <div className="flex items-center">
                                                    <div className="h-10 w-10 flex-shrink-0 rounded-lg bg-primary-100 dark:bg-primary-900/20 flex items-center justify-center text-primary-700 dark:text-primary-400">
                                                        <LinkIcon className="w-5 h-5" />
                                                    </div>
                                                    <div className="ml-4">
                                                        <div className="text-sm font-medium text-gray-900 dark:text-white">{req.name}</div>
                                                        <div className="text-xs text-gray-500 dark:text-gray-400 flex items-center mt-0.5">
                                                            <button
                                                                onClick={() => handleCopy(req.link, req.id)}
                                                                className="flex items-center hover:text-primary-600 dark:hover:text-primary-400"
                                                            >
                                                                {copiedId === req.id ? (
                                                                    <span className="text-green-600 dark:text-green-400 flex items-center">Copied <Check className="w-3 h-3 ml-1" /></span>
                                                                ) : (
                                                                    <span className="flex items-center">Copy Link <Copy className="w-3 h-3 ml-1" /></span>
                                                                )}
                                                            </button>
                                                        </div>
                                                    </div>
                                                </div>
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                {req.destination}
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap">
                                                <span className="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-300">
                                                    {req.upload_count} {req.max_uploads ? `/ ${req.max_uploads}` : ''} files
                                                </span>
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap">
                                                <span className={clsx(
                                                    "px-2 inline-flex text-xs leading-5 font-semibold rounded-full",
                                                    req.status === 'active' ? "bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300" :
                                                        req.status === 'expired' ? "bg-yellow-100 dark:bg-yellow-900/30 text-yellow-800 dark:text-yellow-300" :
                                                            "bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-300"
                                                )}>
                                                    {req.status.charAt(0).toUpperCase() + req.status.slice(1)}
                                                </span>
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400 flex items-center">
                                                <Calendar className="w-4 h-4 mr-2 text-gray-400 dark:text-gray-500" />
                                                {formatDate(req.expires_at)}
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                                <div className="flex items-center justify-end space-x-2">
                                                    <button 
                                                        onClick={() => {
                                                            setSelectedRequest(req);
                                                            setIsDetailsModalOpen(true);
                                                        }}
                                                        className="text-gray-400 hover:text-primary-600 dark:hover:text-primary-400" 
                                                        title="View Details"
                                                    >
                                                        <Eye className="w-5 h-5" />
                                                    </button>
                                                    {req.status === 'active' ? (
                                                        <button
                                                            onClick={() => handleDelete(req.id)}
                                                            className="text-gray-400 hover:text-red-600"
                                                            title="Revoke Link"
                                                        >
                                                            <Trash2 className="w-5 h-5" />
                                                        </button>
                                                    ) : (
                                                        <button
                                                            onClick={() => handlePermanentDelete(req.id)}
                                                            className="text-gray-400 hover:text-red-600"
                                                            title="Permanently Delete"
                                                        >
                                                            <Trash2 className="w-5 h-5" />
                                                        </button>
                                                    )}
                                                </div>
                                            </td>
                                        </tr>
                                    ))}
                                </tbody>
                            </table>
                        </>
                    )}
                </div>
            </div>

            <FilterModal
                isOpen={isFilterOpen}
                onClose={() => setIsFilterOpen(false)}
                onApply={setFilters}
                config={{
                    status: statusFilterOptions,
                    dateFrom: true,
                    dateTo: true,
                }}
                initialValues={filters}
            />

            <CreateFileRequestModal
                isOpen={isCreateModalOpen}
                onClose={() => setIsCreateModalOpen(false)}
                onSubmit={handleCreate}
                defaultVisibility={fileViewMode}
            />

            <FileRequestDetailsModal
                isOpen={isDetailsModalOpen}
                onClose={() => {
                    setIsDetailsModalOpen(false);
                    setSelectedRequest(null);
                }}
                request={selectedRequest}
            />
        </div>
    );
}
