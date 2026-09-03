import { useState, useEffect } from 'react';
import {
    FileText,
    Download,
    Filter,
    Calendar,
    User,
    Activity,
    ChevronLeft,
    ChevronRight,
    RefreshCw,
    AlertCircle,
    CheckCircle,
    Clock,
    Search,
    X
} from 'lucide-react';
import { useAuthFetch, useAuth } from '../context/AuthContext';
import { Navigate } from 'react-router-dom';
import clsx from 'clsx';
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

interface AuditLog {
    id: string;
    user: string;
    user_id: string | null;
    action: string;
    resource: string;
    resource_type: string;
    timestamp: string;
    status: string;
    ip_address: string | null;
    metadata: any;
}

interface AuditLogsResponse {
    logs: AuditLog[];
    total: number;
    limit: number;
    offset: number;
}

interface UserOption {
    id: string;
    name: string;
    email: string;
}

export function AuditLogsPage() {
    const { user } = useAuth();
    const authFetch = useAuthFetch();

    // Only Manager, Admin, and SuperAdmin can access
    if (!user || !['Manager', 'Admin', 'SuperAdmin'].includes(user.role)) {
        return <Navigate to="/" replace />;
    }

    const [logs, setLogs] = useState<AuditLog[]>([]);
    const [total, setTotal] = useState(0);
    const [isLoading, setIsLoading] = useState(true);
    const [isExporting, setIsExporting] = useState(false);

    // Pagination
    const [page, setPage] = useState(1);
    const [limit] = useState(25);

    // Filters
    const [showFilters, setShowFilters] = useState(false);
    const [startDate, setStartDate] = useState('');
    const [endDate, setEndDate] = useState('');
    const [selectedAction, setSelectedAction] = useState('');
    const [selectedUser, setSelectedUser] = useState('');
    const [selectedResourceType, setSelectedResourceType] = useState('');

    // Filter options
    const [actionOptions, setActionOptions] = useState<string[]>([]);
    const [resourceTypeOptions, setResourceTypeOptions] = useState<string[]>([]);
    const [userOptions, setUserOptions] = useState<UserOption[]>([]);

    useEffect(() => {
        fetchLogs();
        fetchFilterOptions();
    }, [page, startDate, endDate, selectedAction, selectedUser, selectedResourceType]);

    const fetchLogs = async () => {
        setIsLoading(true);
        try {
            const offset = (page - 1) * limit;
            let url = `/api/activity-logs?limit=${limit}&offset=${offset}`;

            if (startDate) url += `&start_date=${startDate}`;
            if (endDate) url += `&end_date=${endDate}`;
            if (selectedAction) url += `&action=${encodeURIComponent(selectedAction)}`;
            if (selectedUser) url += `&user_id=${selectedUser}`;
            if (selectedResourceType) url += `&resource_type=${encodeURIComponent(selectedResourceType)}`;

            const response = await authFetch(url);
            if (response.ok) {
                const data: AuditLogsResponse = await response.json();
                setLogs(data.logs);
                setTotal(data.total);
            }
        } catch (error) {
            console.error('Failed to fetch audit logs', error);
        } finally {
            setIsLoading(false);
        }
    };

    const fetchFilterOptions = async () => {
        try {
            // Fetch action types
            const actionsRes = await authFetch('/api/activity-logs/actions');
            if (actionsRes.ok) {
                const data = await actionsRes.json();
                setActionOptions(data.actions || []);
            }

            // Fetch resource types
            const resourcesRes = await authFetch('/api/activity-logs/resource-types');
            if (resourcesRes.ok) {
                const data = await resourcesRes.json();
                setResourceTypeOptions(data.resource_types || []);
            }

            // Fetch users for filtering
            const usersRes = await authFetch('/api/users');
            if (usersRes.ok) {
                const data = await usersRes.json();
                setUserOptions(data.map((u: any) => ({ id: u.id, name: u.name, email: u.email })));
            }
        } catch (error) {
            console.error('Failed to fetch filter options', error);
        }
    };

    const handleExport = async () => {
        setIsExporting(true);
        try {
            let url = '/api/activity-logs/export?';
            const params = [];

            if (startDate) params.push(`start_date=${startDate}`);
            if (endDate) params.push(`end_date=${endDate}`);
            if (selectedAction) params.push(`action=${encodeURIComponent(selectedAction)}`);
            if (selectedUser) params.push(`user_id=${selectedUser}`);
            if (selectedResourceType) params.push(`resource_type=${encodeURIComponent(selectedResourceType)}`);

            url += params.join('&');

            const response = await authFetch(url);
            if (response.ok) {
                const blob = await response.blob();
                const downloadUrl = window.URL.createObjectURL(blob);
                const a = document.createElement('a');
                a.href = downloadUrl;
                a.download = `audit_logs_${new Date().toISOString().split('T')[0]}.csv`;
                document.body.appendChild(a);
                a.click();
                document.body.removeChild(a);
                window.URL.revokeObjectURL(downloadUrl);
            }
        } catch (error) {
            console.error('Failed to export audit logs', error);
        } finally {
            setIsExporting(false);
        }
    };

    const clearFilters = () => {
        setStartDate('');
        setEndDate('');
        setSelectedAction('');
        setSelectedUser('');
        setSelectedResourceType('');
        setPage(1);
    };

    const hasActiveFilters = startDate || endDate || selectedAction || selectedUser || selectedResourceType;

    const totalPages = Math.ceil(total / limit);

    const formatAction = (action: string) => {
        return action
            .replace(/_/g, ' ')
            .replace(/\b\w/g, l => l.toUpperCase());
    };

    const formatTimestamp = (timestamp: string) => {
        const date = new Date(timestamp);
        return new Intl.DateTimeFormat('en-US', {
            month: 'short',
            day: 'numeric',
            year: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
        }).format(date);
    };

    const getStatusIcon = (status: string) => {
        switch (status) {
            case 'warning':
                return <AlertCircle className="w-4 h-4 text-amber-500" />;
            case 'error':
                return <AlertCircle className="w-4 h-4 text-red-500" />;
            default:
                return <CheckCircle className="w-4 h-4 text-emerald-500" />;
        }
    };

    return (
        <div className="p-4 md:p-6 max-w-7xl mx-auto">
            {/* Header */}
            <div className="mb-6">
                <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
                    <div>
                        <h1 className="text-2xl sm:text-3xl font-bold text-foreground flex items-center gap-3">
                            <Activity className="w-7 h-7 text-primary" />
                            Audit Logs
                        </h1>
                        <p className="mt-1 text-sm text-muted-foreground">
                            Track and monitor all activity across your organization
                        </p>
                    </div>
                    <div className="flex items-center gap-2 sm:gap-3">
                        <Button
                            variant={showFilters || hasActiveFilters ? "default" : "outline"}
                            size="sm"
                            onClick={() => setShowFilters(!showFilters)}
                            className="h-9 gap-1.5"
                        >
                            <Filter className="w-4 h-4" />
                            <span>Filters</span>
                            {hasActiveFilters && (
                                <Badge variant="secondary" className="ml-1 px-1.5 py-0 text-xs">
                                    Active
                                </Badge>
                            )}
                        </Button>
                        {['Admin', 'SuperAdmin'].includes(user?.role || '') && (
                            <Button
                                size="sm"
                                onClick={handleExport}
                                disabled={isExporting}
                                className="h-9 gap-1.5"
                            >
                                <Download className="w-4 h-4" />
                                <span>{isExporting ? 'Exporting...' : 'Export CSV'}</span>
                            </Button>
                        )}
                    </div>
                </div>
            </div>

            {/* Filters Panel */}
            {showFilters && (
                <div className="mb-6 bg-card rounded-xl border border-border p-5 shadow-xs">
                    <div className="flex items-center justify-between mb-4">
                        <h3 className="text-sm font-semibold text-foreground">Filter Logs</h3>
                        {hasActiveFilters && (
                            <Button
                                variant="ghost"
                                size="sm"
                                onClick={clearFilters}
                                className="h-8 text-xs text-muted-foreground hover:text-foreground"
                            >
                                <X className="w-3 h-3 mr-1" />
                                Clear all
                            </Button>
                        )}
                    </div>
                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
                        <div>
                            <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                                Start Date
                            </label>
                            <Input
                                type="date"
                                value={startDate}
                                onChange={(e) => { setStartDate(e.target.value); setPage(1); }}
                                className="h-9 text-sm"
                            />
                        </div>
                        <div>
                            <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                                End Date
                            </label>
                            <Input
                                type="date"
                                value={endDate}
                                onChange={(e) => { setEndDate(e.target.value); setPage(1); }}
                                className="h-9 text-sm"
                            />
                        </div>
                        <div>
                            <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                                Action Type
                            </label>
                            <select
                                value={selectedAction}
                                onChange={(e) => { setSelectedAction(e.target.value); setPage(1); }}
                                className="w-full h-9 px-3 text-sm border border-input rounded-md focus:outline-none focus:ring-1 focus:ring-ring bg-background text-foreground"
                            >
                                <option value="">All Actions</option>
                                {actionOptions.map(action => (
                                    <option key={action} value={action}>{formatAction(action)}</option>
                                ))}
                            </select>
                        </div>
                        <div>
                            <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                                User
                            </label>
                            <select
                                value={selectedUser}
                                onChange={(e) => { setSelectedUser(e.target.value); setPage(1); }}
                                className="w-full h-9 px-3 text-sm border border-input rounded-md focus:outline-none focus:ring-1 focus:ring-ring bg-background text-foreground"
                            >
                                <option value="">All Users</option>
                                {userOptions.map(u => (
                                    <option key={u.id} value={u.id}>{u.name}</option>
                                ))}
                            </select>
                        </div>
                        <div>
                            <label className="block text-xs font-medium text-muted-foreground mb-1.5">
                                Resource Type
                            </label>
                            <select
                                value={selectedResourceType}
                                onChange={(e) => { setSelectedResourceType(e.target.value); setPage(1); }}
                                className="w-full h-9 px-3 text-sm border border-input rounded-md focus:outline-none focus:ring-1 focus:ring-ring bg-background text-foreground"
                            >
                                <option value="">All Resources</option>
                                {resourceTypeOptions.map(type => (
                                    <option key={type} value={type}>{type}</option>
                                ))}
                            </select>
                        </div>
                    </div>
                </div>
            )}

            {/* Stats Summary */}
            <div className="mb-4 flex items-center justify-between">
                <p className="text-xs sm:text-sm text-muted-foreground">
                    Showing {logs.length} of {total} total logs
                </p>
                <Button
                    variant="ghost"
                    size="sm"
                    onClick={fetchLogs}
                    disabled={isLoading}
                    className="h-8 gap-1.5 text-xs text-muted-foreground"
                >
                    <RefreshCw className={clsx("w-3.5 h-3.5", isLoading && "animate-spin")} />
                    <span>Refresh</span>
                </Button>
            </div>

            {/* Logs Table */}
            <div className="bg-card rounded-xl border border-border shadow-xs overflow-hidden">
                <div className="overflow-x-auto">
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHead className="w-16">Status</TableHead>
                                <TableHead className="hidden md:table-cell">Timestamp</TableHead>
                                <TableHead>User</TableHead>
                                <TableHead>Action</TableHead>
                                <TableHead className="hidden lg:table-cell">Resource</TableHead>
                                <TableHead className="hidden xl:table-cell">IP Address</TableHead>
                            </TableRow>
                        </TableHeader>
                        <TableBody>
                            {isLoading ? (
                                <TableRow>
                                    <TableCell colSpan={6} className="h-32 text-center">
                                        <div className="flex items-center justify-center">
                                            <div className="animate-spin rounded-full h-7 w-7 border-2 border-primary border-t-transparent"></div>
                                        </div>
                                    </TableCell>
                                </TableRow>
                            ) : logs.length === 0 ? (
                                <TableRow>
                                    <TableCell colSpan={6} className="h-40 text-center text-muted-foreground">
                                        <FileText className="w-10 h-10 mx-auto mb-2 text-muted-foreground/40" />
                                        <p className="text-base font-medium text-foreground">No audit logs found</p>
                                        <p className="text-xs mt-1">Try adjusting your filters or check back later</p>
                                    </TableCell>
                                </TableRow>
                            ) : (
                                logs.map((log) => (
                                    <TableRow key={log.id}>
                                        <TableCell>
                                            {getStatusIcon(log.status)}
                                        </TableCell>
                                        <TableCell className="hidden md:table-cell whitespace-nowrap text-muted-foreground text-xs">
                                            <div className="flex items-center">
                                                <Clock className="w-3 h-3 mr-1.5" />
                                                {formatTimestamp(log.timestamp)}
                                            </div>
                                        </TableCell>
                                        <TableCell className="whitespace-nowrap">
                                            <div className="flex items-center">
                                                <div className="h-7 w-7 rounded-full bg-primary/10 flex items-center justify-center text-primary font-semibold text-xs">
                                                    {log.user.charAt(0).toUpperCase()}
                                                </div>
                                                <div className="ml-2.5">
                                                    <span className="text-sm font-medium text-foreground block">
                                                        {log.user}
                                                    </span>
                                                    <span className="text-xs text-muted-foreground md:hidden">
                                                        {formatTimestamp(log.timestamp)}
                                                    </span>
                                                </div>
                                            </div>
                                        </TableCell>
                                        <TableCell className="whitespace-nowrap">
                                            <Badge variant="secondary" className="font-normal text-xs">
                                                {formatAction(log.action)}
                                            </Badge>
                                        </TableCell>
                                        <TableCell className="hidden lg:table-cell whitespace-nowrap">
                                            <div className="text-sm">
                                                <span className="text-foreground">{log.resource}</span>
                                                <span className="text-muted-foreground text-xs ml-1">
                                                    ({log.resource_type})
                                                </span>
                                            </div>
                                        </TableCell>
                                        <TableCell className="hidden xl:table-cell whitespace-nowrap text-xs text-muted-foreground">
                                            {log.ip_address || '—'}
                                        </TableCell>
                                    </TableRow>
                                ))
                            )}
                        </TableBody>
                    </Table>
                </div>

                {/* Pagination */}
                {totalPages > 1 && (
                    <div className="px-4 py-3 border-t border-border flex items-center justify-between">
                        <p className="text-xs text-muted-foreground">
                            Page {page} of {totalPages}
                        </p>
                        <div className="flex items-center gap-1.5">
                            <Button
                                variant="outline"
                                size="icon"
                                onClick={() => setPage(p => Math.max(1, p - 1))}
                                disabled={page === 1}
                                className="h-8 w-8"
                            >
                                <ChevronLeft className="w-4 h-4" />
                            </Button>
                            <Button
                                variant="outline"
                                size="icon"
                                onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                                disabled={page === totalPages}
                                className="h-8 w-8"
                            >
                                <ChevronRight className="w-4 h-4" />
                            </Button>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}

