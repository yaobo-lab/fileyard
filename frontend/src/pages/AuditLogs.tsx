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
            <div className="mb-8">
                <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
                    <div>
                        <h1 className="text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-3">
                            <Activity className="w-8 h-8 text-primary-600" />
                            Audit Logs
                        </h1>
                        <p className="mt-2 text-gray-600 dark:text-gray-400">
                            Track and monitor all activity across your organization
                        </p>
                    </div>
                    <div className="flex items-center gap-3">
                        <button
                            onClick={() => setShowFilters(!showFilters)}
                            className={clsx(
                                "flex items-center px-4 py-2 rounded-lg text-sm font-medium transition-colors",
                                showFilters || hasActiveFilters
                                    ? "bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-400"
                                    : "bg-white dark:bg-gray-800 text-gray-700 dark:text-gray-200 border border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700"
                            )}
                        >
                            <Filter className="w-4 h-4 mr-2" />
                            Filters
                            {hasActiveFilters && (
                                <span className="ml-2 bg-primary-600 text-white text-xs rounded-full px-2 py-0.5">
                                    Active
                                </span>
                            )}
                        </button>
                        {['Admin', 'SuperAdmin'].includes(user?.role || '') && (
                            <button
                                onClick={handleExport}
                                disabled={isExporting}
                                className="flex items-center px-4 py-2 bg-primary-600 text-white rounded-lg text-sm font-medium hover:bg-primary-700 disabled:opacity-50 transition-colors"
                            >
                                <Download className="w-4 h-4 mr-2" />
                                {isExporting ? 'Exporting...' : 'Export CSV'}
                            </button>
                        )}
                    </div>
                </div>
            </div>

            {/* Filters Panel */}
            {showFilters && (
                <div className="mb-6 bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 p-6 shadow-sm">
                    <div className="flex items-center justify-between mb-4">
                        <h3 className="text-sm font-semibold text-gray-900 dark:text-white">Filter Logs</h3>
                        {hasActiveFilters && (
                            <button
                                onClick={clearFilters}
                                className="text-sm text-primary-600 hover:text-primary-700 flex items-center"
                            >
                                <X className="w-3 h-3 mr-1" />
                                Clear all
                            </button>
                        )}
                    </div>
                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4">
                        <div>
                            <label className="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">
                                Start Date
                            </label>
                            <input
                                type="date"
                                value={startDate}
                                onChange={(e) => { setStartDate(e.target.value); setPage(1); }}
                                className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            />
                        </div>
                        <div>
                            <label className="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">
                                End Date
                            </label>
                            <input
                                type="date"
                                value={endDate}
                                onChange={(e) => { setEndDate(e.target.value); setPage(1); }}
                                className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            />
                        </div>
                        <div>
                            <label className="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">
                                Action Type
                            </label>
                            <select
                                value={selectedAction}
                                onChange={(e) => { setSelectedAction(e.target.value); setPage(1); }}
                                className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            >
                                <option value="">All Actions</option>
                                {actionOptions.map(action => (
                                    <option key={action} value={action}>{formatAction(action)}</option>
                                ))}
                            </select>
                        </div>
                        <div>
                            <label className="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">
                                User
                            </label>
                            <select
                                value={selectedUser}
                                onChange={(e) => { setSelectedUser(e.target.value); setPage(1); }}
                                className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            >
                                <option value="">All Users</option>
                                {userOptions.map(u => (
                                    <option key={u.id} value={u.id}>{u.name}</option>
                                ))}
                            </select>
                        </div>
                        <div>
                            <label className="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">
                                Resource Type
                            </label>
                            <select
                                value={selectedResourceType}
                                onChange={(e) => { setSelectedResourceType(e.target.value); setPage(1); }}
                                className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
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
            <div className="mb-6 flex items-center justify-between">
                <p className="text-sm text-gray-500 dark:text-gray-400">
                    Showing {logs.length} of {total} total logs
                </p>
                <button
                    onClick={fetchLogs}
                    disabled={isLoading}
                    className="flex items-center text-sm text-gray-600 dark:text-gray-400 hover:text-primary-600"
                >
                    <RefreshCw className={clsx("w-4 h-4 mr-1", isLoading && "animate-spin")} />
                    Refresh
                </button>
            </div>

            {/* Logs Table */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
                <div className="overflow-x-auto">
                    <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                        <thead className="bg-gray-50 dark:bg-gray-900/50">
                            <tr>
                                <th className="px-3 md:px-6 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                    Status
                                </th>
                                <th className="hidden md:table-cell px-6 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                    Timestamp
                                </th>
                                <th className="px-3 md:px-6 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                    User
                                </th>
                                <th className="px-3 md:px-6 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                    Action
                                </th>
                                <th className="hidden lg:table-cell px-6 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                    Resource
                                </th>
                                <th className="hidden xl:table-cell px-6 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                    IP Address
                                </th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                            {isLoading ? (
                                <tr>
                                    <td colSpan={6} className="px-6 py-12 text-center">
                                        <div className="flex items-center justify-center">
                                            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
                                        </div>
                                    </td>
                                </tr>
                            ) : logs.length === 0 ? (
                                <tr>
                                    <td colSpan={6} className="px-6 py-12 text-center text-gray-500 dark:text-gray-400">
                                        <FileText className="w-12 h-12 mx-auto mb-3 text-gray-300 dark:text-gray-600" />
                                        <p className="text-lg font-medium">No audit logs found</p>
                                        <p className="text-sm mt-1">Try adjusting your filters or check back later</p>
                                    </td>
                                </tr>
                            ) : (
                                logs.map((log) => (
                                    <tr key={log.id} className="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                                        <td className="px-3 md:px-6 py-4 whitespace-nowrap">
                                            {getStatusIcon(log.status)}
                                        </td>
                                        <td className="hidden md:table-cell px-6 py-4 whitespace-nowrap">
                                            <div className="flex items-center text-sm text-gray-500 dark:text-gray-400">
                                                <Clock className="w-3 h-3 mr-1.5" />
                                                {formatTimestamp(log.timestamp)}
                                            </div>
                                        </td>
                                        <td className="px-3 md:px-6 py-4 whitespace-nowrap">
                                            <div className="flex items-center">
                                                <div className="h-7 w-7 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center text-primary-700 dark:text-primary-400 font-medium text-xs">
                                                    {log.user.charAt(0).toUpperCase()}
                                                </div>
                                                <div className="ml-2">
                                                    <span className="text-sm font-medium text-gray-900 dark:text-white block">
                                                        {log.user}
                                                    </span>
                                                    <span className="text-xs text-gray-400 md:hidden">
                                                        {formatTimestamp(log.timestamp)}
                                                    </span>
                                                </div>
                                            </div>
                                        </td>
                                        <td className="px-3 md:px-6 py-4 whitespace-nowrap">
                                            <span className="inline-flex items-center px-2 md:px-2.5 py-0.5 rounded-full text-xs font-medium bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200">
                                                {formatAction(log.action)}
                                            </span>
                                        </td>
                                        <td className="hidden lg:table-cell px-6 py-4 whitespace-nowrap">
                                            <div className="text-sm">
                                                <span className="text-gray-900 dark:text-white">{log.resource}</span>
                                                <span className="text-gray-400 dark:text-gray-500 ml-1">
                                                    ({log.resource_type})
                                                </span>
                                            </div>
                                        </td>
                                        <td className="hidden xl:table-cell px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                            {log.ip_address || '—'}
                                        </td>
                                    </tr>
                                ))
                            )}
                        </tbody>
                    </table>
                </div>

                {/* Pagination */}
                {totalPages > 1 && (
                    <div className="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex items-center justify-between">
                        <p className="text-sm text-gray-500 dark:text-gray-400">
                            Page {page} of {totalPages}
                        </p>
                        <div className="flex items-center gap-2">
                            <button
                                onClick={() => setPage(p => Math.max(1, p - 1))}
                                disabled={page === 1}
                                className="p-2 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                <ChevronLeft className="w-4 h-4" />
                            </button>
                            <button
                                onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                                disabled={page === totalPages}
                                className="p-2 rounded-lg border border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                <ChevronRight className="w-4 h-4" />
                            </button>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}

