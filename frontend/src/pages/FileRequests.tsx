import { useState, useEffect, useRef } from 'react';
import { Search, Filter, Link as LinkIcon, Calendar, Trash2, Eye, Copy, Check, Plus, Users, EyeOff, ChevronDown } from 'lucide-react';
import clsx from 'clsx';
import { useAuthFetch } from '../context/AuthContext';
import { useGlobalSettings } from '../context/GlobalSettingsContext';
import { FilterModal } from '../components/FilterModal';
import { CreateFileRequestModal, FileRequestData } from '../components/CreateFileRequestModal';
import { FileRequestDetailsModal } from '../components/FileRequestDetailsModal';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';

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
                        <Button
                            variant="outline"
                            size="sm"
                            onClick={(e) => { e.stopPropagation(); setIsViewModeOpen(!isViewModeOpen); }}
                            className="h-9 gap-1.5"
                        >
                            {fileViewMode === 'department' ? (
                                <><Users className="w-4 h-4 text-primary" /><span className="hidden sm:inline">Department Requests</span></>
                            ) : (
                                <><EyeOff className="w-4 h-4 text-purple-500" /><span className="hidden sm:inline">My Private Requests</span></>
                            )}
                            <ChevronDown className="w-3.5 h-3.5 text-muted-foreground/60" />
                        </Button>
                        {isViewModeOpen && (
                            <div className="absolute right-0 mt-2 w-56 bg-popover rounded-xl shadow-lg py-1 z-50 border border-border">
                                <button
                                    onClick={() => { setFileViewMode('department'); setIsViewModeOpen(false); }}
                                    className={clsx(
                                        "flex items-center w-full px-4 py-2 text-sm text-foreground hover:bg-muted transition-colors",
                                        fileViewMode === 'department' && "bg-muted font-medium"
                                    )}
                                >
                                    <Users className="w-4 h-4 mr-3 text-primary" />
                                    Department Requests
                                    {fileViewMode === 'department' && <span className="ml-auto text-primary">✓</span>}
                                </button>
                                <button
                                    onClick={() => { setFileViewMode('private'); setIsViewModeOpen(false); }}
                                    className={clsx(
                                        "flex items-center w-full px-4 py-2 text-sm text-foreground hover:bg-muted transition-colors",
                                        fileViewMode === 'private' && "bg-muted font-medium"
                                    )}
                                >
                                    <EyeOff className="w-4 h-4 mr-3 text-purple-500" />
                                    My Private Requests
                                    {fileViewMode === 'private' && <span className="ml-auto text-primary">✓</span>}
                                </button>
                            </div>
                        )}
                    </div>
                    <Button
                        size="sm"
                        onClick={() => setIsCreateModalOpen(true)}
                        className="h-9 gap-1.5"
                    >
                        <Plus className="w-4 h-4" />
                        <span className="hidden sm:inline">New Request</span>
                    </Button>
                </div>
            </div>

            <div className="bg-card border border-border rounded-xl shadow-xs overflow-hidden transition-colors">
                {/* Toolbar */}
                <div className="p-3 sm:p-4 border-b border-border flex items-center justify-between gap-3 bg-muted/20">
                    <div className="relative max-w-md w-full">
                        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                            <Search className="h-4 w-4 text-muted-foreground/60" />
                        </div>
                        <Input
                            type="text"
                            className="pl-9 h-9 text-sm"
                            placeholder="Search requests..."
                            value={searchTerm}
                            onChange={(e) => setSearchTerm(e.target.value)}
                        />
                    </div>
                    <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setIsFilterOpen(true)}
                        className="h-9 gap-1.5 shrink-0"
                    >
                        <Filter className="w-4 h-4 text-muted-foreground" />
                        <span>Filters</span>
                        {(filters.status || filters.dateFrom || filters.dateTo) && (
                            <span className="ml-1 w-2 h-2 bg-primary rounded-full"></span>
                        )}
                    </Button>
                </div>

                {/* Table */}
                <div className="overflow-x-auto">
                    {isLoading ? (
                        <div className="p-8 text-center text-muted-foreground text-sm">Loading...</div>
                    ) : filteredRequests.length === 0 ? (
                        <div className="p-8 text-center text-muted-foreground text-sm">No file requests found</div>
                    ) : (
                        <>
                            {/* Mobile: Card view */}
                            <div className="sm:hidden divide-y divide-border">
                                {filteredRequests.map((req) => (
                                    <div 
                                        key={req.id} 
                                        className="p-4 hover:bg-muted/30 transition-colors"
                                        onClick={() => {
                                            setSelectedRequest(req);
                                            setIsDetailsModalOpen(true);
                                        }}
                                    >
                                        <div className="flex items-start justify-between">
                                            <div className="flex items-center min-w-0 flex-1">
                                                <div className="h-9 w-9 rounded-lg bg-primary/10 flex items-center justify-center text-primary flex-shrink-0">
                                                    <LinkIcon className="w-4 h-4" />
                                                </div>
                                                <div className="ml-3 min-w-0 flex-1">
                                                    <p className="text-sm font-semibold text-foreground truncate">{req.name}</p>
                                                    <p className="text-xs text-muted-foreground truncate mt-0.5">{req.destination}</p>
                                                    <div className="flex items-center gap-2 mt-2 flex-wrap">
                                                        <Badge variant="secondary" className="text-xs">
                                                            {req.upload_count} {req.max_uploads ? `/ ${req.max_uploads}` : ''} files
                                                        </Badge>
                                                        <Badge
                                                            variant={req.status === 'active' ? 'outline' : 'secondary'}
                                                            className={clsx(
                                                                "text-xs",
                                                                req.status === 'active' && "text-emerald-600 dark:text-emerald-400 border-emerald-500/20"
                                                            )}
                                                        >
                                                            {req.status.charAt(0).toUpperCase() + req.status.slice(1)}
                                                        </Badge>
                                                    </div>
                                                </div>
                                            </div>
                                            <div className="flex items-center gap-1 ml-2" onClick={e => e.stopPropagation()}>
                                                <Button
                                                    variant="ghost"
                                                    size="icon"
                                                    onClick={() => handleCopy(req.link, req.id)}
                                                    className="h-8 w-8 text-muted-foreground"
                                                >
                                                    {copiedId === req.id ? <Check className="w-4 h-4 text-emerald-500" /> : <Copy className="w-4 h-4" />}
                                                </Button>
                                                {req.status === 'active' ? (
                                                    <Button
                                                        variant="ghost"
                                                        size="icon"
                                                        onClick={() => handleDelete(req.id)}
                                                        className="h-8 w-8 text-muted-foreground hover:text-destructive"
                                                        title="Revoke Link"
                                                    >
                                                        <Trash2 className="w-4 h-4" />
                                                    </Button>
                                                ) : (
                                                    <Button
                                                        variant="ghost"
                                                        size="icon"
                                                        onClick={() => handlePermanentDelete(req.id)}
                                                        className="h-8 w-8 text-muted-foreground hover:text-destructive"
                                                        title="Permanently Delete"
                                                    >
                                                        <Trash2 className="w-4 h-4" />
                                                    </Button>
                                                )}
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                            
                            {/* Desktop: Table view */}
                            <Table className="hidden sm:table">
                                <TableHeader>
                                    <TableRow>
                                        <TableHead>Request Name</TableHead>
                                        <TableHead>Destination</TableHead>
                                        <TableHead>Uploads</TableHead>
                                        <TableHead>Status</TableHead>
                                        <TableHead>Expires</TableHead>
                                        <TableHead className="text-right">Actions</TableHead>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {filteredRequests.map((req) => (
                                        <TableRow key={req.id}>
                                            <TableCell className="font-medium">
                                                <div className="flex items-center">
                                                    <div className="h-9 w-9 flex-shrink-0 rounded-lg bg-primary/10 flex items-center justify-center text-primary">
                                                        <LinkIcon className="w-4 h-4" />
                                                    </div>
                                                    <div className="ml-3">
                                                        <div className="text-sm font-semibold text-foreground">{req.name}</div>
                                                        <div className="text-xs text-muted-foreground flex items-center mt-0.5">
                                                            <button
                                                                onClick={() => handleCopy(req.link, req.id)}
                                                                className="flex items-center hover:text-foreground transition-colors cursor-pointer"
                                                            >
                                                                {copiedId === req.id ? (
                                                                    <span className="text-emerald-600 dark:text-emerald-400 flex items-center">Copied <Check className="w-3 h-3 ml-1" /></span>
                                                                ) : (
                                                                    <span className="flex items-center">Copy Link <Copy className="w-3 h-3 ml-1" /></span>
                                                                )}
                                                            </button>
                                                        </div>
                                                    </div>
                                                </div>
                                            </TableCell>
                                            <TableCell className="text-sm text-muted-foreground">
                                                {req.destination}
                                            </TableCell>
                                            <TableCell>
                                                <Badge variant="secondary" className="font-normal text-xs">
                                                    {req.upload_count} {req.max_uploads ? `/ ${req.max_uploads}` : ''} files
                                                </Badge>
                                            </TableCell>
                                            <TableCell>
                                                <Badge
                                                    variant={req.status === 'active' ? 'outline' : 'secondary'}
                                                    className={clsx(
                                                        "text-xs",
                                                        req.status === 'active' && "text-emerald-600 dark:text-emerald-400 border-emerald-500/20"
                                                    )}
                                                >
                                                    {req.status.charAt(0).toUpperCase() + req.status.slice(1)}
                                                </Badge>
                                            </TableCell>
                                            <TableCell className="text-xs text-muted-foreground">
                                                <div className="flex items-center">
                                                    <Calendar className="w-3.5 h-3.5 mr-1.5 text-muted-foreground/60" />
                                                    {formatDate(req.expires_at)}
                                                </div>
                                            </TableCell>
                                            <TableCell className="text-right">
                                                <div className="flex items-center justify-end space-x-1">
                                                    <Button 
                                                        variant="ghost"
                                                        size="icon"
                                                        onClick={() => {
                                                            setSelectedRequest(req);
                                                            setIsDetailsModalOpen(true);
                                                        }}
                                                        className="h-8 w-8 text-muted-foreground"
                                                        title="View Details"
                                                    >
                                                        <Eye className="w-4 h-4" />
                                                    </Button>
                                                    {req.status === 'active' ? (
                                                        <Button
                                                            variant="ghost"
                                                            size="icon"
                                                            onClick={() => handleDelete(req.id)}
                                                            className="h-8 w-8 text-muted-foreground hover:text-destructive"
                                                            title="Revoke Link"
                                                        >
                                                            <Trash2 className="w-4 h-4" />
                                                        </Button>
                                                    ) : (
                                                        <Button
                                                            variant="ghost"
                                                            size="icon"
                                                            onClick={() => handlePermanentDelete(req.id)}
                                                            className="h-8 w-8 text-muted-foreground hover:text-destructive"
                                                            title="Permanently Delete"
                                                        >
                                                            <Trash2 className="w-4 h-4" />
                                                        </Button>
                                                    )}
                                                </div>
                                            </TableCell>
                                        </TableRow>
                                    ))}
                                </TableBody>
                            </Table>
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
