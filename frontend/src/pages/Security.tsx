import { useState, useEffect, useCallback } from 'react';
import {
    Shield,
    AlertTriangle,
    AlertCircle,
    AlertOctagon,
    Info,
    CheckCircle,
    X,
    RefreshCw,
    Filter,
    Clock,
    User,
    Building,
    Globe,
    Download,
    Lock,
    Share2,
    LogIn,
    UserX,
    FileWarning,
    ChevronLeft,
    ChevronRight,
    Trash2,
    CheckSquare,
    Square,
    MinusSquare,
} from 'lucide-react';
import clsx from 'clsx';
import { useAuth, useAuthFetch } from '../context/AuthContext';

interface SecurityAlert {
    id: string;
    tenant_id: string | null;
    tenant_name: string | null;
    user_id: string | null;
    user_email: string | null;
    alert_type: string;
    severity: 'critical' | 'high' | 'medium' | 'low';
    title: string;
    description: string | null;
    metadata: Record<string, unknown>;
    ip_address: string | null;
    resolved: boolean;
    resolved_by: string | null;
    resolved_by_email: string | null;
    resolved_at: string | null;
    created_at: string;
}

interface AlertStats {
    total: number;
    critical: number;
    high: number;
    medium: number;
    low: number;
    unresolved: number;
    by_type: { alert_type: string; count: number }[];
}

const severityConfig = {
    critical: {
        color: 'text-red-600 dark:text-red-400',
        bg: 'bg-red-100 dark:bg-red-900/30',
        border: 'border-red-200 dark:border-red-800',
        icon: AlertOctagon,
        label: 'Critical',
    },
    high: {
        color: 'text-orange-600 dark:text-orange-400',
        bg: 'bg-orange-100 dark:bg-orange-900/30',
        border: 'border-orange-200 dark:border-orange-800',
        icon: AlertTriangle,
        label: 'High',
    },
    medium: {
        color: 'text-yellow-600 dark:text-yellow-400',
        bg: 'bg-yellow-100 dark:bg-yellow-900/30',
        border: 'border-yellow-200 dark:border-yellow-800',
        icon: AlertCircle,
        label: 'Medium',
    },
    low: {
        color: 'text-blue-600 dark:text-blue-400',
        bg: 'bg-blue-100 dark:bg-blue-900/30',
        border: 'border-blue-200 dark:border-blue-800',
        icon: Info,
        label: 'Low',
    },
};

const alertTypeConfig: Record<string, { icon: typeof Shield; label: string }> = {
    failed_login_spike: { icon: LogIn, label: 'Failed Login Spike' },
    new_ip_login: { icon: Globe, label: 'New IP Login' },
    permission_escalation: { icon: Lock, label: 'Permission Escalation' },
    suspended_access_attempt: { icon: UserX, label: 'Suspended User Access' },
    bulk_download: { icon: Download, label: 'Bulk Download' },
    blocked_extension_attempt: { icon: FileWarning, label: 'Blocked Extension' },
    excessive_sharing: { icon: Share2, label: 'Excessive Sharing' },
    account_lockout: { icon: Lock, label: 'Account Lockout' },
    potential_token_theft: { icon: Shield, label: 'Potential Token Theft' },
};

const ITEMS_PER_PAGE = 20;

export function Security() {
    const { user } = useAuth();
    const authFetch = useAuthFetch();
    const [alerts, setAlerts] = useState<SecurityAlert[]>([]);
    const [stats, setStats] = useState<AlertStats | null>(null);
    const [loading, setLoading] = useState(true);
    const [selectedAlert, setSelectedAlert] = useState<SecurityAlert | null>(null);
    const [filterSeverity, setFilterSeverity] = useState<string>('');
    const [filterType, setFilterType] = useState<string>('');
    const [filterResolved, setFilterResolved] = useState<boolean | null>(null);
    const [refreshing, setRefreshing] = useState(false);
    
    // Pagination state
    const [currentPage, setCurrentPage] = useState(1);
    const [totalAlerts, setTotalAlerts] = useState(0);
    
    // Bulk selection state
    const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
    const [bulkLoading, setBulkLoading] = useState(false);

    const isSuperAdmin = user?.role === 'SuperAdmin';
    const totalPages = Math.ceil(totalAlerts / ITEMS_PER_PAGE);

    const fetchAlerts = useCallback(async () => {
        try {
            const params = new URLSearchParams();
            if (filterSeverity) params.set('severity', filterSeverity);
            if (filterType) params.set('alert_type', filterType);
            if (filterResolved !== null) params.set('resolved', String(filterResolved));
            params.set('limit', String(ITEMS_PER_PAGE));
            params.set('offset', String((currentPage - 1) * ITEMS_PER_PAGE));

            const response = await authFetch(`/api/security/alerts?${params.toString()}`);
            if (response.ok) {
                const data = await response.json();
                setAlerts(data.alerts);
                setTotalAlerts(data.total);
            }
        } catch (error) {
            console.error('Failed to fetch alerts:', error);
        }
    }, [authFetch, filterSeverity, filterType, filterResolved, currentPage]);

    const fetchStats = useCallback(async () => {
        try {
            const response = await authFetch('/api/security/alerts/stats');
            if (response.ok) {
                const data = await response.json();
                setStats(data);
            }
        } catch (error) {
            console.error('Failed to fetch stats:', error);
        }
    }, [authFetch]);

    const loadData = useCallback(async () => {
        setLoading(true);
        await Promise.all([fetchAlerts(), fetchStats()]);
        setLoading(false);
    }, [fetchAlerts, fetchStats]);

    useEffect(() => {
        loadData();
    }, [loadData]);

    // Reset to page 1 when filters change
    useEffect(() => {
        setCurrentPage(1);
        setSelectedIds(new Set());
    }, [filterSeverity, filterType, filterResolved]);

    const handleRefresh = async () => {
        setRefreshing(true);
        setSelectedIds(new Set());
        await loadData();
        setRefreshing(false);
    };

    const handleResolve = async (alertId: string) => {
        try {
            const response = await authFetch(`/api/security/alerts/${alertId}/resolve`, {
                method: 'POST',
            });
            if (response.ok) {
                await loadData();
                setSelectedAlert(null);
            }
        } catch (error) {
            console.error('Failed to resolve alert:', error);
        }
    };

    const handleDismiss = async (alertId: string) => {
        if (!confirm('Are you sure you want to dismiss this alert? This action cannot be undone.')) {
            return;
        }
        try {
            const response = await authFetch(`/api/security/alerts/${alertId}/dismiss`, {
                method: 'POST',
            });
            if (response.ok) {
                await loadData();
                setSelectedAlert(null);
            }
        } catch (error) {
            console.error('Failed to dismiss alert:', error);
        }
    };

    // Bulk action handlers
    const handleBulkAction = async (action: 'resolve' | 'dismiss') => {
        if (selectedIds.size === 0) return;
        
        const confirmMessage = action === 'resolve' 
            ? `Mark ${selectedIds.size} alert(s) as resolved?`
            : `Dismiss ${selectedIds.size} alert(s)? This cannot be undone.`;
        
        if (!confirm(confirmMessage)) return;

        setBulkLoading(true);
        try {
            const response = await authFetch('/api/security/alerts/bulk', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    ids: Array.from(selectedIds),
                    action,
                }),
            });
            if (response.ok) {
                const data = await response.json();
                console.log(`Bulk ${action}:`, data);
                setSelectedIds(new Set());
                await loadData();
            }
        } catch (error) {
            console.error(`Failed to bulk ${action}:`, error);
        } finally {
            setBulkLoading(false);
        }
    };

    const toggleSelect = (id: string, event: React.MouseEvent) => {
        event.stopPropagation();
        setSelectedIds(prev => {
            const next = new Set(prev);
            if (next.has(id)) {
                next.delete(id);
            } else {
                next.add(id);
            }
            return next;
        });
    };

    const toggleSelectAll = () => {
        if (selectedIds.size === alerts.length) {
            setSelectedIds(new Set());
        } else {
            setSelectedIds(new Set(alerts.map(a => a.id)));
        }
    };

    const formatDate = (dateStr: string) => {
        const date = new Date(dateStr);
        return date.toLocaleString();
    };

    const getAlertTypeInfo = (type: string) => {
        return alertTypeConfig[type] || { icon: Shield, label: type.replace(/_/g, ' ') };
    };

    // Pagination helpers
    const goToPage = (page: number) => {
        if (page >= 1 && page <= totalPages) {
            setCurrentPage(page);
            setSelectedIds(new Set());
        }
    };

    const getPageNumbers = () => {
        const pages: (number | string)[] = [];
        if (totalPages <= 7) {
            for (let i = 1; i <= totalPages; i++) pages.push(i);
        } else {
            if (currentPage <= 3) {
                pages.push(1, 2, 3, 4, '...', totalPages);
            } else if (currentPage >= totalPages - 2) {
                pages.push(1, '...', totalPages - 3, totalPages - 2, totalPages - 1, totalPages);
            } else {
                pages.push(1, '...', currentPage - 1, currentPage, currentPage + 1, '...', totalPages);
            }
        }
        return pages;
    };

    if (loading) {
        return (
            <div className="flex items-center justify-center min-h-[60vh]">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
            </div>
        );
    }

    return (
        <div className="space-y-4 sm:space-y-6">
            {/* Header */}
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 sm:gap-4">
                <div className="flex items-center gap-2 sm:gap-3">
                    <Shield className="w-6 h-6 sm:w-8 sm:h-8 text-primary-600" />
                    <div>
                        <h1 className="text-xl sm:text-2xl font-bold text-gray-900 dark:text-white">
                            Security Alerts
                        </h1>
                        <p className="text-xs sm:text-sm text-gray-500 dark:text-gray-400">
                            {isSuperAdmin ? 'Monitor security events across all companies' : 'Monitor security events for your company'}
                        </p>
                    </div>
                </div>
                <button
                    onClick={handleRefresh}
                    disabled={refreshing}
                    className="flex items-center justify-center gap-2 px-3 sm:px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 self-start sm:self-auto"
                >
                    <RefreshCw className={clsx('w-4 h-4', refreshing && 'animate-spin')} />
                    <span className="hidden sm:inline">Refresh</span>
                </button>
            </div>

            {/* Stats Cards */}
            {stats && (
                <div className="grid grid-cols-2 md:grid-cols-5 gap-4">
                    <div className="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700">
                        <div className="flex items-center justify-between">
                            <span className="text-sm text-gray-500 dark:text-gray-400">Total</span>
                            <Shield className="w-5 h-5 text-gray-400" />
                        </div>
                        <div className="mt-2 text-2xl font-bold text-gray-900 dark:text-white">
                            {stats.total}
                        </div>
                    </div>
                    <div className={clsx('rounded-xl p-4 border', severityConfig.critical.bg, severityConfig.critical.border)}>
                        <div className="flex items-center justify-between">
                            <span className={clsx('text-sm', severityConfig.critical.color)}>Critical</span>
                            <AlertOctagon className={clsx('w-5 h-5', severityConfig.critical.color)} />
                        </div>
                        <div className={clsx('mt-2 text-2xl font-bold', severityConfig.critical.color)}>
                            {stats.critical}
                        </div>
                    </div>
                    <div className={clsx('rounded-xl p-4 border', severityConfig.high.bg, severityConfig.high.border)}>
                        <div className="flex items-center justify-between">
                            <span className={clsx('text-sm', severityConfig.high.color)}>High</span>
                            <AlertTriangle className={clsx('w-5 h-5', severityConfig.high.color)} />
                        </div>
                        <div className={clsx('mt-2 text-2xl font-bold', severityConfig.high.color)}>
                            {stats.high}
                        </div>
                    </div>
                    <div className={clsx('rounded-xl p-4 border', severityConfig.medium.bg, severityConfig.medium.border)}>
                        <div className="flex items-center justify-between">
                            <span className={clsx('text-sm', severityConfig.medium.color)}>Medium</span>
                            <AlertCircle className={clsx('w-5 h-5', severityConfig.medium.color)} />
                        </div>
                        <div className={clsx('mt-2 text-2xl font-bold', severityConfig.medium.color)}>
                            {stats.medium}
                        </div>
                    </div>
                    <div className={clsx('rounded-xl p-4 border', severityConfig.low.bg, severityConfig.low.border)}>
                        <div className="flex items-center justify-between">
                            <span className={clsx('text-sm', severityConfig.low.color)}>Low</span>
                            <Info className={clsx('w-5 h-5', severityConfig.low.color)} />
                        </div>
                        <div className={clsx('mt-2 text-2xl font-bold', severityConfig.low.color)}>
                            {stats.low}
                        </div>
                    </div>
                </div>
            )}

            {/* Filters */}
            <div className="bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700">
                <div className="flex items-center gap-2 mb-3">
                    <Filter className="w-4 h-4 text-gray-400" />
                    <span className="text-sm font-medium text-gray-700 dark:text-gray-300">Filters</span>
                </div>
                <div className="flex flex-wrap gap-3">
                    <select
                        value={filterSeverity}
                        onChange={(e) => setFilterSeverity(e.target.value)}
                        className="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                    >
                        <option value="">All Severities</option>
                        <option value="critical">Critical</option>
                        <option value="high">High</option>
                        <option value="medium">Medium</option>
                        <option value="low">Low</option>
                    </select>
                    <select
                        value={filterType}
                        onChange={(e) => setFilterType(e.target.value)}
                        className="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                    >
                        <option value="">All Types</option>
                        {Object.entries(alertTypeConfig).map(([type, config]) => (
                            <option key={type} value={type}>{config.label}</option>
                        ))}
                    </select>
                    <select
                        value={filterResolved === null ? '' : String(filterResolved)}
                        onChange={(e) => setFilterResolved(e.target.value === '' ? null : e.target.value === 'true')}
                        className="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                    >
                        <option value="">All Status</option>
                        <option value="false">Unresolved</option>
                        <option value="true">Resolved</option>
                    </select>
                    {(filterSeverity || filterType || filterResolved !== null) && (
                        <button
                            onClick={() => {
                                setFilterSeverity('');
                                setFilterType('');
                                setFilterResolved(null);
                            }}
                            className="px-3 py-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
                        >
                            Clear Filters
                        </button>
                    )}
                </div>
            </div>

            {/* Bulk Actions Bar */}
            {selectedIds.size > 0 && (
                <div className="bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 rounded-xl p-4 flex items-center justify-between">
                    <span className="text-sm font-medium text-primary-700 dark:text-primary-300">
                        {selectedIds.size} alert{selectedIds.size !== 1 ? 's' : ''} selected
                    </span>
                    <div className="flex items-center gap-2">
                        <button
                            onClick={() => handleBulkAction('resolve')}
                            disabled={bulkLoading}
                            className="flex items-center gap-2 px-3 py-1.5 bg-green-600 text-white text-sm rounded-lg hover:bg-green-700 disabled:opacity-50"
                        >
                            <CheckCircle className="w-4 h-4" />
                            Resolve Selected
                        </button>
                        <button
                            onClick={() => handleBulkAction('dismiss')}
                            disabled={bulkLoading}
                            className="flex items-center gap-2 px-3 py-1.5 bg-red-600 text-white text-sm rounded-lg hover:bg-red-700 disabled:opacity-50"
                        >
                            <Trash2 className="w-4 h-4" />
                            Dismiss Selected
                        </button>
                        <button
                            onClick={() => setSelectedIds(new Set())}
                            className="px-3 py-1.5 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
                        >
                            Clear Selection
                        </button>
                    </div>
                </div>
            )}

            {/* Alerts List */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden">
                {alerts.length === 0 ? (
                    <div className="p-8 text-center">
                        <Shield className="w-12 h-12 mx-auto text-gray-300 dark:text-gray-600 mb-3" />
                        <p className="text-gray-500 dark:text-gray-400">No security alerts found</p>
                    </div>
                ) : (
                    <>
                        {/* Select All Header */}
                        <div className="px-4 py-3 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50">
                            <button
                                onClick={toggleSelectAll}
                                className="flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
                            >
                                {selectedIds.size === 0 ? (
                                    <Square className="w-4 h-4" />
                                ) : selectedIds.size === alerts.length ? (
                                    <CheckSquare className="w-4 h-4 text-primary-600" />
                                ) : (
                                    <MinusSquare className="w-4 h-4 text-primary-600" />
                                )}
                                {selectedIds.size === alerts.length ? 'Deselect All' : 'Select All'}
                            </button>
                        </div>
                        
                        <div className="divide-y divide-gray-200 dark:divide-gray-700">
                            {alerts.map((alert) => {
                                const sevConfig = severityConfig[alert.severity];
                                const typeInfo = getAlertTypeInfo(alert.alert_type);
                                const TypeIcon = typeInfo.icon;
                                const SevIcon = sevConfig.icon;
                                const isSelected = selectedIds.has(alert.id);

                                return (
                                    <div
                                        key={alert.id}
                                        className={clsx(
                                            'p-4 hover:bg-gray-50 dark:hover:bg-gray-700/50 cursor-pointer transition-colors',
                                            alert.resolved && 'opacity-60',
                                            isSelected && 'bg-primary-50 dark:bg-primary-900/20'
                                        )}
                                        onClick={() => setSelectedAlert(alert)}
                                    >
                                        <div className="flex items-start gap-4">
                                            {/* Checkbox */}
                                            <button
                                                onClick={(e) => toggleSelect(alert.id, e)}
                                                className="mt-1 text-gray-400 hover:text-primary-600"
                                            >
                                                {isSelected ? (
                                                    <CheckSquare className="w-5 h-5 text-primary-600" />
                                                ) : (
                                                    <Square className="w-5 h-5" />
                                                )}
                                            </button>
                                            
                                            <div className={clsx('p-2 rounded-lg', sevConfig.bg)}>
                                                <SevIcon className={clsx('w-5 h-5', sevConfig.color)} />
                                            </div>
                                            <div className="flex-1 min-w-0">
                                                <div className="flex items-center gap-2 mb-1">
                                                    <span className={clsx(
                                                        'px-2 py-0.5 rounded text-xs font-medium',
                                                        sevConfig.bg, sevConfig.color
                                                    )}>
                                                        {sevConfig.label}
                                                    </span>
                                                    <span className="flex items-center gap-1 text-xs text-gray-500 dark:text-gray-400">
                                                        <TypeIcon className="w-3 h-3" />
                                                        {typeInfo.label}
                                                    </span>
                                                    {alert.resolved && (
                                                        <span className="flex items-center gap-1 text-xs text-green-600 dark:text-green-400">
                                                            <CheckCircle className="w-3 h-3" />
                                                            Resolved
                                                        </span>
                                                    )}
                                                </div>
                                                <h3 className="font-medium text-gray-900 dark:text-white truncate">
                                                    {alert.title}
                                                </h3>
                                                <p className="text-sm text-gray-500 dark:text-gray-400 truncate">
                                                    {alert.description}
                                                </p>
                                                <div className="flex items-center gap-4 mt-2 text-xs text-gray-400 dark:text-gray-500">
                                                    <span className="flex items-center gap-1">
                                                        <Clock className="w-3 h-3" />
                                                        {formatDate(alert.created_at)}
                                                    </span>
                                                    {alert.user_email && (
                                                        <span className="flex items-center gap-1">
                                                            <User className="w-3 h-3" />
                                                            {alert.user_email}
                                                        </span>
                                                    )}
                                                    {isSuperAdmin && alert.tenant_name && (
                                                        <span className="flex items-center gap-1">
                                                            <Building className="w-3 h-3" />
                                                            {alert.tenant_name}
                                                        </span>
                                                    )}
                                                    {alert.ip_address && (
                                                        <span className="flex items-center gap-1">
                                                            <Globe className="w-3 h-3" />
                                                            {alert.ip_address}
                                                        </span>
                                                    )}
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                );
                            })}
                        </div>
                    </>
                )}
            </div>

            {/* Pagination */}
            {totalPages > 1 && (
                <div className="flex flex-col sm:flex-row items-center justify-between gap-4 bg-white dark:bg-gray-800 rounded-xl p-4 border border-gray-200 dark:border-gray-700">
                    <div className="text-sm text-gray-500 dark:text-gray-400">
                        Showing {((currentPage - 1) * ITEMS_PER_PAGE) + 1} to {Math.min(currentPage * ITEMS_PER_PAGE, totalAlerts)} of {totalAlerts} alerts
                    </div>
                    <div className="flex items-center gap-1">
                        <button
                            onClick={() => goToPage(currentPage - 1)}
                            disabled={currentPage === 1}
                            className="p-2 rounded-lg text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            <ChevronLeft className="w-5 h-5" />
                        </button>
                        
                        {getPageNumbers().map((page, index) => (
                            typeof page === 'number' ? (
                                <button
                                    key={index}
                                    onClick={() => goToPage(page)}
                                    className={clsx(
                                        'px-3 py-1.5 rounded-lg text-sm font-medium',
                                        currentPage === page
                                            ? 'bg-primary-600 text-white'
                                            : 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700'
                                    )}
                                >
                                    {page}
                                </button>
                            ) : (
                                <span key={index} className="px-2 text-gray-400">...</span>
                            )
                        ))}
                        
                        <button
                            onClick={() => goToPage(currentPage + 1)}
                            disabled={currentPage === totalPages}
                            className="p-2 rounded-lg text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            <ChevronRight className="w-5 h-5" />
                        </button>
                    </div>
                </div>
            )}

            {/* Alert Detail Modal */}
            {selectedAlert && (
                <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
                    <div className="bg-white dark:bg-gray-800 rounded-xl max-w-2xl w-full max-h-[90vh] overflow-auto">
                        <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                            <div className="flex items-center justify-between">
                                <div className="flex items-center gap-3">
                                    {(() => {
                                        const sevConfig = severityConfig[selectedAlert.severity];
                                        const SevIcon = sevConfig.icon;
                                        return (
                                            <div className={clsx('p-2 rounded-lg', sevConfig.bg)}>
                                                <SevIcon className={clsx('w-6 h-6', sevConfig.color)} />
                                            </div>
                                        );
                                    })()}
                                    <div>
                                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                                            {selectedAlert.title}
                                        </h2>
                                        <div className="flex items-center gap-2 mt-1">
                                            <span className={clsx(
                                                'px-2 py-0.5 rounded text-xs font-medium',
                                                severityConfig[selectedAlert.severity].bg,
                                                severityConfig[selectedAlert.severity].color
                                            )}>
                                                {severityConfig[selectedAlert.severity].label}
                                            </span>
                                            <span className="text-xs text-gray-500 dark:text-gray-400">
                                                {getAlertTypeInfo(selectedAlert.alert_type).label}
                                            </span>
                                        </div>
                                    </div>
                                </div>
                                <button
                                    onClick={() => setSelectedAlert(null)}
                                    className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200"
                                >
                                    <X className="w-5 h-5" />
                                </button>
                            </div>
                        </div>
                        <div className="p-6 space-y-4">
                            {selectedAlert.description && (
                                <div>
                                    <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Description</h3>
                                    <p className="text-gray-600 dark:text-gray-400">{selectedAlert.description}</p>
                                </div>
                            )}
                            <div className="grid grid-cols-2 gap-4">
                                <div>
                                    <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Time</h3>
                                    <p className="text-gray-600 dark:text-gray-400">{formatDate(selectedAlert.created_at)}</p>
                                </div>
                                {selectedAlert.user_email && (
                                    <div>
                                        <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">User</h3>
                                        <p className="text-gray-600 dark:text-gray-400">{selectedAlert.user_email}</p>
                                    </div>
                                )}
                                {isSuperAdmin && selectedAlert.tenant_name && (
                                    <div>
                                        <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Company</h3>
                                        <p className="text-gray-600 dark:text-gray-400">{selectedAlert.tenant_name}</p>
                                    </div>
                                )}
                                {selectedAlert.ip_address && (
                                    <div>
                                        <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">IP Address</h3>
                                        <p className="text-gray-600 dark:text-gray-400">{selectedAlert.ip_address}</p>
                                    </div>
                                )}
                            </div>
                            {Object.keys(selectedAlert.metadata).length > 0 && (
                                <div>
                                    <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">Details</h3>
                                    <div className="bg-gray-50 dark:bg-gray-900/50 rounded-lg p-4 space-y-2">
                                        {Object.entries(selectedAlert.metadata).map(([key, value]) => (
                                            <div key={key} className="flex justify-between text-sm">
                                                <span className="text-gray-500 dark:text-gray-400">
                                                    {key.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())}
                                                </span>
                                                <span className="text-gray-900 dark:text-white font-mono">
                                                    {typeof value === 'object' ? JSON.stringify(value) : String(value)}
                                                </span>
                                            </div>
                                        ))}
                                    </div>
                                </div>
                            )}
                            {selectedAlert.resolved && (
                                <div className="bg-green-50 dark:bg-green-900/20 rounded-lg p-4">
                                    <div className="flex items-center gap-2 text-green-700 dark:text-green-400">
                                        <CheckCircle className="w-5 h-5" />
                                        <span className="font-medium">Resolved</span>
                                    </div>
                                    {selectedAlert.resolved_by_email && (
                                        <p className="text-sm text-green-600 dark:text-green-500 mt-1">
                                            By {selectedAlert.resolved_by_email} on {selectedAlert.resolved_at ? formatDate(selectedAlert.resolved_at) : 'unknown'}
                                        </p>
                                    )}
                                </div>
                            )}
                        </div>
                        {!selectedAlert.resolved && (
                            <div className="p-6 border-t border-gray-200 dark:border-gray-700 flex gap-3">
                                <button
                                    onClick={() => handleResolve(selectedAlert.id)}
                                    className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700"
                                >
                                    <CheckCircle className="w-4 h-4" />
                                    Mark as Resolved
                                </button>
                                <button
                                    onClick={() => handleDismiss(selectedAlert.id)}
                                    className="flex-1 flex items-center justify-center gap-2 px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300 rounded-lg hover:bg-gray-300 dark:hover:bg-gray-600"
                                >
                                    <X className="w-4 h-4" />
                                    Dismiss (False Positive)
                                </button>
                            </div>
                        )}
                    </div>
                </div>
            )}
        </div>
    );
}
