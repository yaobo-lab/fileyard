import { useState, useEffect, useCallback } from 'react';
import { Trash2, RefreshCw, ArrowLeft, AlertCircle, Filter, Loader2, RotateCcw, Trash, Building2, User } from 'lucide-react';
import { Link } from 'react-router-dom';
import clsx from 'clsx';

import { useTenant } from '../context/TenantContext';
import { useGlobalSettings } from '../context/GlobalSettingsContext';
import { useAuthFetch, useAuth } from '../context/AuthContext';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card } from '@/components/ui/card';

interface TrashItem {
    id: string;
    name: string;
    size: string;
    size_bytes?: number;
    modified: string | null;
    deleted_at: string | null;
    original_path: string;
    owner_name?: string;
    owner_id?: string;
}

interface DepartmentOption {
    id: string;
    name: string;
}

export default function RecycleBin() {
    const [items, setItems] = useState<TrashItem[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [selectedDepartment, setSelectedDepartment] = useState<string>('all');
    const [departments, setDepartments] = useState<DepartmentOption[]>([]);
    const [isRestoring, setIsRestoring] = useState<string | null>(null);
    const [isDeleting, setIsDeleting] = useState<string | null>(null);
    
    const { currentCompany } = useTenant();
    const { formatDate } = useGlobalSettings();
    const authFetch = useAuthFetch();
    const { user } = useAuth();
    
    const companyId = currentCompany?.id;
    const isAdmin = user?.role === 'SuperAdmin' || user?.role === 'Admin';

    // Fetch departments for filter dropdown (admins only)
    useEffect(() => {
        if (!companyId || !isAdmin) return;
        
        const fetchDepartments = async () => {
            try {
                const response = await authFetch('/api/departments');
                if (response.ok) {
                    const data = await response.json();
                    setDepartments(data.map((d: any) => ({
                        id: d.id,
                        name: d.name,
                    })));
                }
            } catch (err) {
                console.error('Failed to fetch departments for filter', err);
            }
        };
        fetchDepartments();
    }, [companyId, isAdmin, authFetch]);

    const fetchTrash = useCallback(async () => {
        if (!companyId) return;
        
        setIsLoading(true);
        setError(null);
        
        try {
            // Build URL with department filter for admins
            let url = `/api/trash/${companyId}`;
            if (isAdmin && selectedDepartment !== 'all') {
                url += `?department_id=${selectedDepartment}`;
            }
            
            const response = await authFetch(url);
            
            if (!response.ok) {
                if (response.status === 401) {
                    throw new Error('Session expired. Please log in again.');
                } else if (response.status === 403) {
                    throw new Error('You do not have permission to view the recycle bin.');
                } else {
                    throw new Error(`Failed to fetch recycle bin (${response.status})`);
                }
            }
            
            const data = await response.json();
            setItems(data || []);
        } catch (err) {
            console.error('Failed to fetch trash', err);
            setError(err instanceof Error ? err.message : 'Failed to load recycle bin');
        } finally {
            setIsLoading(false);
        }
    }, [companyId, authFetch, isAdmin, selectedDepartment]);

    useEffect(() => {
        if (companyId) {
            fetchTrash();
        }
    }, [companyId, fetchTrash]);

    const handleRestore = async (item: TrashItem) => {
        if (!companyId) return;
        
        setIsRestoring(item.id);
        try {
            const response = await authFetch(`/api/trash/${companyId}/restore/${encodeURIComponent(item.name)}`, {
                method: 'POST'
            });
            
            if (!response.ok) {
                throw new Error('Failed to restore file');
            }
            
            // Remove from local state immediately for snappy UI
            setItems(prev => prev.filter(i => i.id !== item.id));
        } catch (err) {
            console.error('Failed to restore file', err);
            setError('Failed to restore file. Please try again.');
        } finally {
            setIsRestoring(null);
        }
    };

    const handleDelete = async (item: TrashItem) => {
        if (!companyId) return;
        if (!confirm(`Permanently delete "${item.name}"? This cannot be undone.`)) return;
        
        setIsDeleting(item.id);
        try {
            const response = await authFetch(`/api/trash/${companyId}/delete/${encodeURIComponent(item.name)}`, {
                method: 'POST'
            });
            
            if (!response.ok) {
                throw new Error('Failed to delete file');
            }
            
            // Remove from local state immediately
            setItems(prev => prev.filter(i => i.id !== item.id));
        } catch (err) {
            console.error('Failed to delete file', err);
            setError('Failed to permanently delete file. Please try again.');
        } finally {
            setIsDeleting(null);
        }
    };

    const formatFileSize = (bytes?: number): string => {
        if (!bytes) return 'Unknown size';
        if (bytes < 1024) return `${bytes} B`;
        if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
        if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
        return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
    };

    const safeFormatDate = (dateStr: string | null | undefined): string => {
        if (!dateStr) return 'Unknown date';
        try {
            return formatDate(dateStr);
        } catch {
            return 'Invalid date';
        }
    };

    return (
        <div className="h-full flex flex-col space-y-4">
            {/* Header */}
            <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 mb-4">
                <div className="flex items-center space-x-2 md:space-x-3">
                    <Button variant="ghost" size="icon" asChild className="h-9 w-9">
                        <Link to="/files">
                            <ArrowLeft className="w-4 h-4" />
                        </Link>
                    </Button>
                    <div>
                        <h1 className="text-xl md:text-2xl font-bold tracking-tight text-foreground flex items-center">
                            <Trash2 className="w-5 h-5 mr-2 text-destructive" />
                            Recycle Bin
                        </h1>
                        <p className="text-xs text-muted-foreground mt-0.5 hidden sm:block">
                            Items will be permanently deleted after {currentCompany?.retention_policy_days || 30} days.
                        </p>
                    </div>
                </div>
                
                <div className="flex items-center justify-between sm:justify-end gap-2 sm:gap-3">
                    {/* Department filter for admins */}
                    {isAdmin && departments.length > 0 && (
                        <div className="flex items-center gap-1.5 sm:gap-2 flex-1 sm:flex-none">
                            <Building2 className="w-4 h-4 text-muted-foreground hidden sm:block" />
                            <select
                                value={selectedDepartment}
                                onChange={(e) => setSelectedDepartment(e.target.value)}
                                className="text-sm border border-input rounded-md px-2.5 py-1.5 bg-background text-foreground focus:ring-1 focus:ring-ring w-full sm:w-auto h-9"
                            >
                                <option value="all">All Departments</option>
                                {departments.map(d => (
                                    <option key={d.id} value={d.id}>{d.name}</option>
                                ))}
                            </select>
                        </div>
                    )}
                    
                    <Button
                        variant="outline"
                        size="icon"
                        onClick={fetchTrash}
                        disabled={isLoading}
                        className="h-9 w-9 shrink-0"
                        title="Refresh"
                    >
                        <RefreshCw className={clsx("w-4 h-4", isLoading && "animate-spin")} />
                    </Button>
                </div>
            </div>

            {/* Error Alert */}
            {error && (
                <div className="bg-destructive/10 border border-destructive/20 rounded-xl p-4 flex items-start space-x-3 text-destructive">
                    <AlertCircle className="w-5 h-5 flex-shrink-0 mt-0.5" />
                    <div className="flex-1">
                        <p className="text-sm font-medium">{error}</p>
                    </div>
                    <button
                        onClick={() => setError(null)}
                        className="hover:opacity-80 text-lg leading-none"
                    >
                        ×
                    </button>
                </div>
            )}

            {/* Content */}
            {isLoading ? (
                <div className="text-center py-20">
                    <Loader2 className="w-10 h-10 text-primary mx-auto animate-spin" />
                    <p className="mt-3 text-sm text-muted-foreground">Loading recycle bin...</p>
                </div>
            ) : items.length === 0 ? (
                <Card className="text-center py-16 shadow-xs">
                    <Trash2 className="w-12 h-12 text-muted-foreground/40 mx-auto mb-3" />
                    <p className="text-muted-foreground text-sm font-medium">
                        {selectedDepartment !== 'all' ? 'No deleted files for this department' : 'Recycle bin is empty'}
                    </p>
                    {selectedDepartment !== 'all' && (
                        <Button
                            variant="link"
                            size="sm"
                            onClick={() => setSelectedDepartment('all')}
                            className="mt-2 text-primary"
                        >
                            Show all departments
                        </Button>
                    )}
                </Card>
            ) : (
                <div className="bg-card shadow-xs overflow-hidden rounded-xl border border-border">
                    {/* Table Header for Admins - Hidden on mobile */}
                    {isAdmin && (
                        <div className="hidden md:grid px-6 py-3 bg-muted/40 border-b border-border grid-cols-12 gap-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                            <div className="col-span-5">File</div>
                            <div className="col-span-2">Owner</div>
                            <div className="col-span-2">Deleted</div>
                            <div className="col-span-3 text-right">Actions</div>
                        </div>
                    )}
                    
                    <div className="divide-y divide-border">
                        {items.map((item) => (
                            <div 
                                key={item.id} 
                                className={clsx(
                                    "px-4 md:px-6 py-3.5 md:py-4 hover:bg-muted/30 transition-colors group",
                                    isAdmin ? "flex flex-col md:grid md:grid-cols-12 gap-3 md:gap-4 md:items-center" : "flex flex-col md:flex-row md:items-center md:justify-between gap-3"
                                )}
                            >
                                {/* File Info */}
                                <div className={clsx("flex items-center min-w-0 w-full", isAdmin ? "md:col-span-5" : "md:flex-1")}>
                                    <div className="flex-shrink-0 h-9 w-9 md:h-10 md:w-10 rounded-lg bg-destructive/10 flex items-center justify-center">
                                        <Trash2 className="h-4 w-4 md:h-5 md:w-5 text-destructive" />
                                    </div>
                                    <div className="ml-3 min-w-0 flex-1">
                                        <div className="text-sm font-semibold text-foreground truncate" title={item.name}>
                                            {item.name}
                                        </div>
                                        <div className="text-xs text-muted-foreground mt-0.5 truncate">
                                            {item.size || formatFileSize(item.size_bytes)}
                                            {item.original_path && (
                                                <span className="ml-2 text-muted-foreground/60 hidden sm:inline">
                                                    from {item.original_path}
                                                </span>
                                            )}
                                        </div>
                                    </div>
                                </div>
                                
                                {/* Owner & Date Row - Stacked on mobile, grid columns on desktop */}
                                {isAdmin ? (
                                    <div className="flex items-center justify-between md:contents text-xs md:text-sm text-muted-foreground pl-12 md:pl-0">
                                        {/* Owner */}
                                        <div className="flex items-center md:col-span-2">
                                            <User className="w-3.5 h-3.5 mr-1.5 text-muted-foreground/60" />
                                            <span className="truncate">{item.owner_name || 'Unknown'}</span>
                                        </div>
                                        {/* Deleted Date */}
                                        <div className="md:col-span-2 text-xs">
                                        {safeFormatDate(item.deleted_at || item.modified)}
                                        </div>
                                    </div>
                                ) : (
                                    <div className="text-xs text-muted-foreground pl-12 md:pl-0 md:mx-4">
                                        Deleted {safeFormatDate(item.deleted_at || item.modified)}
                                    </div>
                                )}
                                
                                {/* Actions - Full width on mobile */}
                                <div className={clsx("flex gap-2 w-full md:w-auto", isAdmin ? "md:col-span-3 md:justify-end" : "md:justify-end")}>
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        onClick={() => handleRestore(item)}
                                        disabled={isRestoring === item.id}
                                        className="flex-1 md:flex-none h-8 gap-1 text-emerald-600 hover:text-emerald-700 hover:bg-emerald-500/10 border-emerald-500/20"
                                    >
                                        {isRestoring === item.id ? (
                                            <Loader2 className="w-3.5 h-3.5 animate-spin" />
                                        ) : (
                                            <RotateCcw className="w-3.5 h-3.5" />
                                        )}
                                        <span>Restore</span>
                                    </Button>
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        onClick={() => handleDelete(item)}
                                        disabled={isDeleting === item.id}
                                        className="flex-1 md:flex-none h-8 gap-1 text-destructive hover:text-destructive hover:bg-destructive/10 border-destructive/20"
                                    >
                                        {isDeleting === item.id ? (
                                            <Loader2 className="w-3.5 h-3.5 animate-spin" />
                                        ) : (
                                            <Trash className="w-3.5 h-3.5" />
                                        )}
                                        <span>Delete</span>
                                    </Button>
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            )}
            
            {/* Item count */}
            {!isLoading && items.length > 0 && (
                <p className="text-sm text-gray-500 dark:text-gray-400 text-center">
                    {items.length} {items.length === 1 ? 'item' : 'items'} in recycle bin
                </p>
            )}
        </div>
    );
}
