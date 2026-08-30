import { useEffect, useState, useMemo, useRef } from 'react';
import { createSwapy } from 'swapy';
import { ActivityFeed } from '../components/ActivityFeed';
import { RequestSummary } from '../components/RequestSummary';
import { StatCard } from '../components/StatCard';
import { WidgetSettingsModal } from '../components/WidgetSettingsModal';
import { 
    RecentUploadsWidget, 
    ExpiringWidget, 
    ComplianceStatusWidget, 
    QuickStatsWidget, 
    NotificationsWidget, 
    StorageTrendsWidget,
    ActivityChartWidget,
    FileTypesChartWidget
} from '../components/widgets';
import { Building2, Users, HardDrive, ShieldCheck, FileText, FolderOpen, Settings2, AlertTriangle, X, RotateCcw } from 'lucide-react';
import { useSettings } from '../context/SettingsContext';
import { useAuthFetch, useAuth } from '../context/AuthContext';

// Default layout: slot -> widget mapping
const DEFAULT_LAYOUT: Record<string, string> = {
    '1': 'stats-1',
    '2': 'stats-2',
    '3': 'stats-3',
    '4': 'stats-4',
    '5': 'activity-chart',
    '6': 'file-types',
    '7': 'activity',
    '8': 'requests',
    '9': 'departments'
};

interface WidgetConfig {
    visible_widgets: string[];
    widget_settings: Record<string, any>;
    custom_widgets: string[];
}

const DEFAULT_WIDGET_CONFIG: WidgetConfig = {
    visible_widgets: ['stats-1', 'stats-2', 'stats-3', 'stats-4', 'activity-chart', 'file-types', 'activity', 'requests', 'departments'],
    widget_settings: {},
    custom_widgets: []
};

interface DashboardStats {
    companies: number;
    users: number;
    files: number;
    storage_used_bytes: number;
    storage_used_formatted: string;
    storage_quota_bytes: number | null;
    storage_quota_formatted: string | null;
}

export function Dashboard() {
    const { complianceMode } = useSettings();
    const { user, refreshUser, tenant } = useAuth();
    const authFetch = useAuthFetch();
    const containerRef = useRef<HTMLDivElement>(null);
    const swapyRef = useRef<ReturnType<typeof createSwapy> | null>(null);
    
    const [stats, setStats] = useState<DashboardStats>({
        companies: 0,
        users: 0,
        files: 0,
        storage_used_bytes: 0,
        storage_used_formatted: '0 B',
        storage_quota_bytes: null,
        storage_quota_formatted: null
    });
    const [departments, setDepartments] = useState<any[]>([]);
    const [layout, setLayout] = useState<Record<string, string>>(() => {
        // Try to load from user preferences
        if (user?.dashboard_layout && typeof user.dashboard_layout === 'object' && !Array.isArray(user.dashboard_layout)) {
            return user.dashboard_layout as Record<string, string>;
        }
        return DEFAULT_LAYOUT;
    });
    const [widgetConfig, setWidgetConfig] = useState<WidgetConfig>(
        user?.widget_config || DEFAULT_WIDGET_CONFIG
    );
    const [isSettingsOpen, setIsSettingsOpen] = useState(false);
    const [tenantSwitchNotice, setTenantSwitchNotice] = useState<{suspended_tenant: string, current_tenant: string} | null>(null);

    // Check for tenant switch notice (shown when user's primary company is suspended)
    useEffect(() => {
        const notice = sessionStorage.getItem('tenant_switch_notice');
        const dismissed = localStorage.getItem('tenant_switch_notice_dismissed');
        if (notice && !dismissed) {
            try {
                setTenantSwitchNotice(JSON.parse(notice));
                sessionStorage.removeItem('tenant_switch_notice');
            } catch {
                sessionStorage.removeItem('tenant_switch_notice');
            }
        } else if (notice) {
            sessionStorage.removeItem('tenant_switch_notice');
        }
    }, []);

    const dismissTenantNotice = (dontShowAgain: boolean) => {
        if (dontShowAgain) {
            localStorage.setItem('tenant_switch_notice_dismissed', 'true');
        }
        setTenantSwitchNotice(null);
    };

    // Update layout when user data loads
    useEffect(() => {
        if (user?.dashboard_layout && typeof user.dashboard_layout === 'object' && !Array.isArray(user.dashboard_layout)) {
            setLayout(user.dashboard_layout as Record<string, string>);
        }
        if (user?.widget_config) {
            setWidgetConfig(user.widget_config);
        }
    }, [user?.dashboard_layout, user?.widget_config]);

    // Initialize Swapy
    useEffect(() => {
        if (!containerRef.current) return;

        // Destroy existing instance
        if (swapyRef.current) {
            swapyRef.current.destroy();
        }

        // Create new swapy instance
        swapyRef.current = createSwapy(containerRef.current, {
            animation: 'dynamic'
        });

        // Handle swap events
        swapyRef.current.onSwap((event: any) => {
            const newLayout: Record<string, string> = {};
            const swapData = event.data?.array || event.newSlotItemMap?.asArray || [];
            swapData.forEach((item: any) => {
                if (item.item && item.slot) {
                    newLayout[item.slot] = item.item;
                }
            });
            
            // Only update if we got valid data
            if (Object.keys(newLayout).length > 0) {
                setLayout(newLayout);
                
                // Save to backend
                authFetch(`/api/users/${user?.id}`, {
                    method: 'PUT',
                    body: JSON.stringify({ dashboard_layout: newLayout })
                }).catch(error => {
                    console.error('Failed to save layout:', error);
                });
            }
        });

        return () => {
            if (swapyRef.current) {
                swapyRef.current.destroy();
                swapyRef.current = null;
            }
        };
    }, [user?.id, authFetch]);

    // Fetch data on mount and when tenant changes
    useEffect(() => {
        fetchStats();
        fetchDepartments();
    }, [tenant?.id]);

    const fetchStats = async () => {
        try {
            const statsRes = await authFetch('/api/dashboard/stats');
            if (statsRes.ok) {
                const data = await statsRes.json();
                setStats({
                    companies: data.stats?.companies || 0,
                    users: data.stats?.users || 0,
                    files: data.stats?.files || 0,
                    storage_used_bytes: data.stats?.storage_used_bytes || 0,
                    storage_used_formatted: data.stats?.storage_used_formatted || '0 B',
                    storage_quota_bytes: data.stats?.storage_quota_bytes || null,
                    storage_quota_formatted: data.stats?.storage_quota_formatted || null
                });
            }
        } catch (error) {
            console.error('Failed to fetch stats', error);
        }
    };

    const fetchDepartments = async () => {
        try {
            const res = await authFetch('/api/departments');
            if (res.ok) {
                const data = await res.json();
                setDepartments(data);
            }
        } catch (error) {
            console.error('Failed to fetch departments', error);
        }
    };

    const handleWidgetConfigSave = (newConfig: WidgetConfig) => {
        setWidgetConfig(newConfig);
        refreshUser();
    };

    const getWidgetSettings = (widgetId: string) => {
        return widgetConfig.widget_settings[widgetId] || {};
    };

    const departmentSettings = getWidgetSettings('departments');
    const maxDepartments = departmentSettings.max_shown || 6;

    // Widget components map
    const widgets: Record<string, React.ReactNode> = useMemo(() => ({
        'stats-1': (
            <StatCard
                title="Total Companies"
                value={stats.companies.toString()}
                icon={Building2}
            />
        ),
        'stats-2': (
            <StatCard
                title="Active Users"
                value={stats.users.toString()}
                icon={Users}
            />
        ),
        'stats-3': (
            <StatCard
                title="Storage Used"
                value={stats.storage_quota_formatted 
                    ? `${stats.storage_used_formatted} / ${stats.storage_quota_formatted}`
                    : stats.storage_used_formatted}
                icon={HardDrive}
            />
        ),
        'stats-4': (
            <StatCard
                title="Files & Folders"
                value={stats.files.toString()}
                icon={FileText}
            />
        ),
        'activity': <ActivityFeed limit={getWidgetSettings('activity').limit || 10} />,
        'activity-chart': <ActivityChartWidget days={getWidgetSettings('activity-chart').days || 7} />,
        'file-types': <FileTypesChartWidget />,
        'requests': <RequestSummary />,
        'departments': (
            <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-6 h-full overflow-auto">
                <div className="flex items-center justify-between mb-4">
                    <h3 className="text-lg font-medium text-gray-900 dark:text-white">Departments</h3>
                    <span className="text-sm text-gray-500 dark:text-gray-400">{departments.length} Total</span>
                </div>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    {departments.slice(0, maxDepartments).map((dept) => (
                        <div key={dept.id} className="flex items-center p-3 bg-gray-50 dark:bg-gray-700/50 rounded-md border border-gray-100 dark:border-gray-700">
                            <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-full mr-3">
                                <FolderOpen className="w-4 h-4 text-primary-600 dark:text-primary-400" />
                            </div>
                            <div>
                                <p className="text-sm font-medium text-gray-900 dark:text-white">{dept.name}</p>
                                <p className="text-xs text-gray-500 dark:text-gray-400">Active</p>
                            </div>
                        </div>
                    ))}
                    {departments.length === 0 && (
                        <p className="text-sm text-gray-500 dark:text-gray-400 col-span-2 text-center py-4">No departments found</p>
                    )}
                </div>
            </div>
        ),
        // Additional widgets (can be swapped in via Customize)
        'quick-stats': <QuickStatsWidget />,
        'recent-uploads': <RecentUploadsWidget limit={getWidgetSettings('recent-uploads').limit || 5} />,
        'upcoming-expiry': <ExpiringWidget daysAhead={getWidgetSettings('upcoming-expiry').days_ahead || 7} />,
        'compliance-status': <ComplianceStatusWidget />,
        'storage-trends': <StorageTrendsWidget period={getWidgetSettings('storage-trends').period || '30d'} />,
        'notifications': <NotificationsWidget limit={getWidgetSettings('notifications').limit || 5} />
    }), [stats, departments, maxDepartments, widgetConfig]);

    // Get widget for a slot
    const getSlotWidget = (slot: string) => {
        const widgetId = layout[slot];
        if (!widgetId || !widgets[widgetId]) return null;
        return (
            <div data-swapy-item={widgetId} className="h-full">
                {widgets[widgetId]}
            </div>
        );
    };

    // Reset layout to default
    const handleResetLayout = async () => {
        setLayout(DEFAULT_LAYOUT);
        try {
            await authFetch(`/api/users/${user?.id}`, {
                method: 'PUT',
                body: JSON.stringify({ dashboard_layout: DEFAULT_LAYOUT })
            });
            window.location.reload();
        } catch (error) {
            console.error('Failed to reset layout:', error);
        }
    };

    return (
        <div className="space-y-4 sm:space-y-6">
            {/* Tenant Switch Notice */}
            {tenantSwitchNotice && (
                <div className="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg p-4 flex items-start gap-3">
                    <AlertTriangle className="w-5 h-5 text-amber-600 dark:text-amber-400 mt-0.5 flex-shrink-0" />
                    <div className="flex-1">
                        <h3 className="text-sm font-medium text-amber-800 dark:text-amber-200">
                            Primary Company Suspended
                        </h3>
                        <p className="text-sm text-amber-700 dark:text-amber-300 mt-1">
                            Your primary company "{tenantSwitchNotice.suspended_tenant}" has been suspended. 
                            You've been logged into "{tenantSwitchNotice.current_tenant}" instead.
                        </p>
                        <div className="flex items-center gap-4 mt-3">
                            <button 
                                onClick={() => dismissTenantNotice(false)}
                                className="text-xs text-amber-700 dark:text-amber-300 hover:underline"
                            >
                                Dismiss
                            </button>
                            <button 
                                onClick={() => dismissTenantNotice(true)}
                                className="text-xs text-amber-600 dark:text-amber-400 hover:underline"
                            >
                                Don't show again
                            </button>
                        </div>
                    </div>
                    <button 
                        onClick={() => dismissTenantNotice(false)}
                        className="text-amber-600 dark:text-amber-400 hover:text-amber-800 dark:hover:text-amber-200"
                    >
                        <X className="w-5 h-5" />
                    </button>
                </div>
            )}

            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 sm:gap-4 mb-4 sm:mb-6">
                <div>
                    <h1 className="text-xl sm:text-2xl font-bold text-gray-900 dark:text-white">Dashboard</h1>
                    <p className="text-xs sm:text-sm text-gray-500 dark:text-gray-400 mt-0.5 sm:mt-1">
                        {complianceMode === 'None' || complianceMode === 'Standard' ? 'System Overview' : `${complianceMode} Compliance Monitoring`}
                    </p>
                </div>
                <div className="flex items-center gap-2">
                    {complianceMode !== 'None' && complianceMode !== 'Standard' && (
                        <span className="inline-flex items-center px-2 sm:px-3 py-1 rounded-full text-xs font-medium bg-green-100 text-green-800 border border-green-200 dark:bg-green-900/30 dark:text-green-400 dark:border-green-800">
                            <ShieldCheck className="w-3 h-3 sm:mr-1" />
                            <span className="hidden sm:inline">{complianceMode} Compliant</span>
                        </span>
                    )}
                    <button
                        onClick={handleResetLayout}
                        className="inline-flex items-center px-2.5 sm:px-3 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                        title="Reset Layout"
                    >
                        <RotateCcw className="w-4 h-4 sm:mr-2" />
                        <span className="hidden sm:inline">Reset Layout</span>
                    </button>
                    <button
                        onClick={() => setIsSettingsOpen(true)}
                        className="inline-flex items-center px-2.5 sm:px-3 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                        title="Customize"
                    >
                        <Settings2 className="w-4 h-4 sm:mr-2" />
                        <span className="hidden sm:inline">Customize</span>
                    </button>
                </div>
            </div>

            {/* Swapy Container */}
            <div ref={containerRef} className="space-y-6">
                {/* Stats Row */}
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
                    <div data-swapy-slot="1" className="min-h-[100px]">
                        {getSlotWidget('1')}
                    </div>
                    <div data-swapy-slot="2" className="min-h-[100px]">
                        {getSlotWidget('2')}
                    </div>
                    <div data-swapy-slot="3" className="min-h-[100px]">
                        {getSlotWidget('3')}
                    </div>
                    <div data-swapy-slot="4" className="min-h-[100px]">
                        {getSlotWidget('4')}
                    </div>
                </div>

                {/* Charts Row */}
                <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                    <div data-swapy-slot="5" className="min-h-[300px]">
                        {getSlotWidget('5')}
                    </div>
                    <div data-swapy-slot="6" className="min-h-[300px]">
                        {getSlotWidget('6')}
                    </div>
                </div>

                {/* Main Content Row */}
                <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
                    <div data-swapy-slot="7" className="lg:col-span-2 min-h-[400px]">
                        {getSlotWidget('7')}
                    </div>
                    <div className="space-y-6">
                        <div data-swapy-slot="8" className="min-h-[180px]">
                            {getSlotWidget('8')}
                        </div>
                        <div data-swapy-slot="9" className="min-h-[180px]">
                            {getSlotWidget('9')}
                        </div>
                    </div>
                </div>
            </div>

            {/* Widget Settings Modal */}
            <WidgetSettingsModal
                isOpen={isSettingsOpen}
                onClose={() => setIsSettingsOpen(false)}
                onSave={handleWidgetConfigSave}
                currentConfig={widgetConfig}
            />
        </div>
    );
}
