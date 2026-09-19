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
import { useTranslations } from '../context/I18nContext';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';

// Widget categories
const STAT_WIDGET_IDS = ['stats-1', 'stats-2', 'stats-3', 'stats-4'];
const DEFAULT_VISIBLE_WIDGETS = [
    'stats-1', 'stats-2', 'stats-3', 'stats-4',
    'activity-chart', 'file-types', 'activity', 'requests', 'departments'
];

// Default layout mapping
const DEFAULT_LAYOUT: Record<string, string> = {
    'stat-0': 'stats-1',
    'stat-1': 'stats-2',
    'stat-2': 'stats-3',
    'stat-3': 'stats-4',
    'main-0': 'activity-chart',
    'main-1': 'file-types',
    'main-2': 'activity',
    'main-3': 'requests',
    'main-4': 'departments'
};

interface WidgetConfig {
    visible_widgets: string[];
    widget_settings: Record<string, any>;
    custom_widgets: string[];
}

const DEFAULT_WIDGET_CONFIG: WidgetConfig = {
    visible_widgets: DEFAULT_VISIBLE_WIDGETS,
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

// 提取当前可见的 widget 列表，按 layout 已排好的顺序排序，新开启的 widget 追加到末尾
function getOrderedWidgets(
    layout: Record<string, string>,
    visibleWidgets: string[],
    isStat: boolean
): string[] {
    const targetSet = new Set(
        visibleWidgets
            .map(id => id === 'storage' ? 'file-types' : id)
            .filter(id => isStat ? STAT_WIDGET_IDS.includes(id) : !STAT_WIDGET_IDS.includes(id))
    );

    // 提取 layout 中所有已分配槽位的 widgets，按槽位升序排列
    const sortedSlots = Object.keys(layout).sort((a, b) => {
        const numA = parseInt(a.replace(/\D/g, ''), 10) || 0;
        const numB = parseInt(b.replace(/\D/g, ''), 10) || 0;
        return numA - numB;
    });

    const orderedFromLayout: string[] = [];
    sortedSlots.forEach(slot => {
        let widgetId = layout[slot];
        if (widgetId === 'storage') widgetId = 'file-types';
        if (widgetId && targetSet.has(widgetId) && !orderedFromLayout.includes(widgetId)) {
            orderedFromLayout.push(widgetId);
            targetSet.delete(widgetId);
        }
    });

    // 将新启用、尚未在 layout 中的 widget 追加到后面
    const remainingWidgets: string[] = [];
    targetSet.forEach(widgetId => {
        remainingWidgets.push(widgetId);
    });

    return [...orderedFromLayout, ...remainingWidgets];
}

export function Dashboard() {
    const t = useTranslations('Dashboard');
    const tCommon = useTranslations('Common');
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

    // 计算当前可见与排序的小组件
    const visibleWidgets = useMemo(() => {
        const list = widgetConfig?.visible_widgets || DEFAULT_VISIBLE_WIDGETS;
        return list.map(id => id === 'storage' ? 'file-types' : id);
    }, [widgetConfig?.visible_widgets]);

    const orderedStats = useMemo(() => {
        return getOrderedWidgets(layout, visibleWidgets, true);
    }, [layout, visibleWidgets]);

    const orderedMain = useMemo(() => {
        return getOrderedWidgets(layout, visibleWidgets, false);
    }, [layout, visibleWidgets]);

    // Initialize Swapy
    useEffect(() => {
        if (!containerRef.current) return;

        // Destroy existing instance
        if (swapyRef.current) {
            swapyRef.current.destroy();
            swapyRef.current = null;
        }

        // Create new swapy instance
        swapyRef.current = createSwapy(containerRef.current, {
            animation: 'dynamic'
        });

        // Handle swap events
        swapyRef.current.onSwap((event: any) => {
            const swapData = event.data?.array || event.newSlotItemMap?.asArray || [];
            if (!swapData || swapData.length === 0) return;

            setLayout(prevLayout => {
                const newLayout: Record<string, string> = { ...prevLayout };
                swapData.forEach((item: any) => {
                    if (item.item && item.slot) {
                        newLayout[item.slot] = item.item;
                    }
                });

                if (user?.id) {
                    authFetch(`/api/users/${user.id}`, {
                        method: 'PUT',
                        body: JSON.stringify({ dashboard_layout: newLayout })
                    }).catch(error => {
                        console.error('Failed to save layout:', error);
                    });
                }

                return newLayout;
            });
        });

        return () => {
            if (swapyRef.current) {
                swapyRef.current.destroy();
                swapyRef.current = null;
            }
        };
    }, [user?.id, authFetch, orderedStats.join(','), orderedMain.join(',')]);

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
                title={t('totalCompanies')}
                value={stats.companies.toString()}
                icon={Building2}
            />
        ),
        'stats-2': (
            <StatCard
                title={t('activeUsers')}
                value={stats.users.toString()}
                icon={Users}
            />
        ),
        'stats-3': (
            <StatCard
                title={t('storageUsed')}
                value={stats.storage_quota_formatted 
                    ? `${stats.storage_used_formatted} / ${stats.storage_quota_formatted}`
                    : stats.storage_used_formatted}
                icon={HardDrive}
            />
        ),
        'stats-4': (
            <StatCard
                title={t('filesAndFolders')}
                value={stats.files.toString()}
                icon={FileText}
            />
        ),
        'activity': <ActivityFeed limit={getWidgetSettings('activity').limit || getWidgetSettings('activity').max_items || 7} />,
        'activity-chart': <ActivityChartWidget days={getWidgetSettings('activity-chart').days || 7} />,
        'file-types': <FileTypesChartWidget />,
        'requests': <RequestSummary />,
        'departments': (
            <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-6 h-full overflow-auto">
                <div className="flex items-center justify-between mb-4">
                    <h3 className="text-lg font-medium text-gray-900 dark:text-white">{t('departments')}</h3>
                    <span className="text-sm text-gray-500 dark:text-gray-400">{t('departmentsTotal', { count: departments.length })}</span>
                </div>
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                    {departments.slice(0, maxDepartments).map((dept) => (
                        <div key={dept.id} className="flex items-center p-3 bg-gray-50 dark:bg-gray-700/50 rounded-md border border-gray-100 dark:border-gray-700">
                            <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-full mr-3">
                                <FolderOpen className="w-4 h-4 text-primary-600 dark:text-primary-400" />
                            </div>
                            <div>
                                <p className="text-sm font-medium text-gray-900 dark:text-white">{dept.name}</p>
                                <p className="text-xs text-gray-500 dark:text-gray-400">{t('active')}</p>
                            </div>
                        </div>
                    ))}
                    {departments.length === 0 && (
                        <p className="text-sm text-gray-500 dark:text-gray-400 col-span-2 text-center py-4">{t('noDepartments')}</p>
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

    // Reset layout to default
    const handleResetLayout = async () => {
        setLayout(DEFAULT_LAYOUT);
        setWidgetConfig(DEFAULT_WIDGET_CONFIG);
        try {
            await authFetch(`/api/users/${user?.id}`, {
                method: 'PUT',
                body: JSON.stringify({ 
                    dashboard_layout: DEFAULT_LAYOUT,
                    widget_config: DEFAULT_WIDGET_CONFIG
                })
            });
            refreshUser();
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
                            {t('primaryCompanySuspended')}
                        </h3>
                        <p className="text-sm text-amber-700 dark:text-amber-300 mt-1">
                            {t('primaryCompanySuspendedDesc', {
                                suspended: tenantSwitchNotice.suspended_tenant,
                                current: tenantSwitchNotice.current_tenant
                            })}
                        </p>
                        <div className="flex items-center gap-4 mt-3">
                            <button 
                                onClick={() => dismissTenantNotice(false)}
                                className="text-xs text-amber-700 dark:text-amber-300 hover:underline"
                            >
                                {t('dismiss')}
                            </button>
                            <button 
                                onClick={() => dismissTenantNotice(true)}
                                className="text-xs text-amber-600 dark:text-amber-400 hover:underline"
                            >
                                {t('dontShowAgain')}
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
                    <h1 className="text-xl sm:text-2xl font-bold text-foreground">{t('title')}</h1>
                    <p className="text-xs sm:text-sm text-muted-foreground mt-0.5 sm:mt-1">
                        {t('systemOverview')}
                    </p>
                </div>
                <div className="flex items-center gap-2">
                    <Button
                        variant="outline"
                        size="sm"
                        onClick={handleResetLayout}
                        className="h-9 gap-1.5"
                        title={t('resetLayout')}
                    >
                        <RotateCcw className="w-4 h-4 text-muted-foreground" />
                        <span className="hidden sm:inline">{t('resetLayout')}</span>
                    </Button>
                    <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setIsSettingsOpen(true)}
                        className="h-9 gap-1.5"
                        title={t('customize')}
                    >
                        <Settings2 className="w-4 h-4 text-muted-foreground" />
                        <span className="hidden sm:inline">{t('customize')}</span>
                    </Button>
                </div>
            </div>

            {/* Swapy Container */}
            <div ref={containerRef} className="space-y-6">
                {/* Stats Row */}
                {orderedStats.length > 0 && (
                    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
                        {orderedStats.map((widgetId, index) => (
                            <div 
                                key={`stat-slot-${index}`} 
                                data-swapy-slot={`stat-slot-${index}`} 
                                className="min-h-[100px]"
                            >
                                <div data-swapy-item={widgetId} className="h-full">
                                    {widgets[widgetId]}
                                </div>
                            </div>
                        ))}
                    </div>
                )}

                {/* Main Content & Charts Grid */}
                {orderedMain.length > 0 && (
                    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                        {orderedMain.map((widgetId, index) => (
                            <div 
                                key={`main-slot-${index}`} 
                                data-swapy-slot={`main-slot-${index}`} 
                                className="min-h-[320px]"
                            >
                                <div data-swapy-item={widgetId} className="h-full">
                                    {widgets[widgetId]}
                                </div>
                            </div>
                        ))}
                    </div>
                )}

                {/* Empty state when all widgets are hidden */}
                {orderedStats.length === 0 && orderedMain.length === 0 && (
                    <div className="text-center py-16 bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700">
                        <Settings2 className="w-12 h-12 mx-auto mb-4 text-muted-foreground opacity-50" />
                        <h3 className="text-base font-medium text-foreground mb-1">
                            {t('noWidgetsVisible') || '当前未显示任何小组件'}
                        </h3>
                        <p className="text-sm text-muted-foreground mb-4">
                            {t('clickCustomizeToAdd') || '点击“自定义组件”选择要在仪表盘中展示的内容'}
                        </p>
                        <Button 
                            variant="default"
                            onClick={() => setIsSettingsOpen(true)}
                            className="gap-2"
                        >
                            <Settings2 className="w-4 h-4" />
                            {t('customize')}
                        </Button>
                    </div>
                )}
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
