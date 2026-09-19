import { useState, useEffect } from 'react';
import { X, Eye, EyeOff, Settings2, Plus, Building2, Users, HardDrive, FileText, Activity, FolderOpen, Link as LinkIcon, BarChart3, Clock, Calendar, Bell, Shield, TrendingUp } from 'lucide-react';
import { useAuthFetch, useAuth } from '../context/AuthContext';
import { useTranslations } from '../context/I18nContext';

interface WidgetConfig {
    visible_widgets: string[];
    widget_settings: Record<string, any>;
    custom_widgets: string[];
}

interface WidgetDefinition {
    id: string;
    name: string;
    description: string;
    icon: React.ElementType;
    configurable: boolean;
    defaultConfig?: Record<string, any>;
}

// Core widgets that are always available
const CORE_WIDGETS: WidgetDefinition[] = [
    { id: 'stats-1', name: 'Companies', description: 'Total companies count', icon: Building2, configurable: false },
    { id: 'stats-2', name: 'Users', description: 'Active users count', icon: Users, configurable: false },
    { id: 'stats-3', name: 'Storage', description: 'Storage usage summary', icon: HardDrive, configurable: false },
    { id: 'stats-4', name: 'Files', description: 'Total files count', icon: FileText, configurable: false },
    { id: 'activity-chart', name: 'Activity Chart', description: 'Activity overview', icon: BarChart3, configurable: true, defaultConfig: { days: 7 } },
    { id: 'file-types', name: 'File Types', description: 'File types distribution', icon: HardDrive, configurable: false },
    { id: 'activity', name: 'Activity Feed', description: 'Recent activity log', icon: Activity, configurable: true, defaultConfig: { limit: 7, max_items: 7 } },
    { id: 'requests', name: 'File Requests', description: 'Active file request links', icon: LinkIcon, configurable: true, defaultConfig: { show_expired: false } },
    { id: 'departments', name: 'Departments', description: 'Department overview', icon: FolderOpen, configurable: true, defaultConfig: { max_shown: 6 } },
];

// Additional widgets that can be added
const ADDITIONAL_WIDGETS: WidgetDefinition[] = [
    { id: 'quick-stats', name: 'Quick Stats', description: 'Compact statistics overview', icon: BarChart3, configurable: false },
    { id: 'recent-uploads', name: 'Recent Uploads', description: 'Latest uploaded files', icon: Clock, configurable: true, defaultConfig: { limit: 5 } },
    { id: 'upcoming-expiry', name: 'Expiring Soon', description: 'Files and requests expiring soon', icon: Calendar, configurable: true, defaultConfig: { days_ahead: 7 } },
    { id: 'compliance-status', name: 'Compliance Status', description: 'Compliance mode overview', icon: Shield, configurable: false },
    { id: 'storage-trends', name: 'Storage Trends', description: 'Storage usage over time', icon: TrendingUp, configurable: true, defaultConfig: { period: '30d' } },
    { id: 'notifications', name: 'Notifications', description: 'Recent system notifications', icon: Bell, configurable: true, defaultConfig: { limit: 5 } },
];

const AVAILABLE_WIDGETS: WidgetDefinition[] = [...CORE_WIDGETS, ...ADDITIONAL_WIDGETS];

const DEFAULT_CONFIG: WidgetConfig = {
    visible_widgets: ['stats-1', 'stats-2', 'stats-3', 'stats-4', 'activity-chart', 'file-types', 'activity', 'requests', 'departments'],
    widget_settings: {},
    custom_widgets: []
};

interface WidgetSettingsModalProps {
    isOpen: boolean;
    onClose: () => void;
    onSave: (config: WidgetConfig) => void;
    currentConfig?: WidgetConfig;
}

export function WidgetSettingsModal({ isOpen, onClose, onSave, currentConfig }: WidgetSettingsModalProps) {
    const t = useTranslations('Widgets');
    const tCommon = useTranslations('Common');
    const [config, setConfig] = useState<WidgetConfig>(currentConfig || DEFAULT_CONFIG);
    const [activeTab, setActiveTab] = useState<'visibility' | 'settings' | 'add'>('visibility');
    const [isSaving, setIsSaving] = useState(false);
    const { user } = useAuth();
    const authFetch = useAuthFetch();

    useEffect(() => {
        if (currentConfig) {
            setConfig(currentConfig);
        }
    }, [currentConfig]);

    const isWidgetVisible = (widgetId: string) => {
        if (config.visible_widgets.includes(widgetId)) return true;
        if (widgetId === 'file-types' && config.visible_widgets.includes('storage')) return true;
        return false;
    };

    const toggleWidgetVisibility = (widgetId: string) => {
        setConfig(prev => {
            const isCurrentlyVisible = isWidgetVisible(widgetId);
            let nextVisible: string[];
            if (isCurrentlyVisible) {
                nextVisible = prev.visible_widgets.filter(
                    id => id !== widgetId && !(widgetId === 'file-types' && id === 'storage')
                );
            } else {
                const cleaned = prev.visible_widgets.filter(
                    id => !(widgetId === 'file-types' && id === 'storage')
                );
                nextVisible = [...cleaned, widgetId];
            }
            return {
                ...prev,
                visible_widgets: nextVisible
            };
        });
    };

    const updateWidgetSetting = (widgetId: string, key: string, value: any) => {
        setConfig(prev => ({
            ...prev,
            widget_settings: {
                ...prev.widget_settings,
                [widgetId]: {
                    ...prev.widget_settings[widgetId],
                    [key]: value
                }
            }
        }));
    };

    const handleSave = async () => {
        setIsSaving(true);
        try {
            // 规范化 visible_widgets，将旧的 'storage' 替换为 'file-types'
            const normalizedConfig = {
                ...config,
                visible_widgets: config.visible_widgets.map(id => id === 'storage' ? 'file-types' : id)
            };
            // Save to backend
            await authFetch(`/api/users/${user?.id}`, {
                method: 'PUT',
                body: JSON.stringify({ widget_config: normalizedConfig })
            });
            onSave(normalizedConfig);
            onClose();
        } catch (error) {
            console.error('Failed to save widget config:', error);
        } finally {
            setIsSaving(false);
        }
    };

    const handleReset = () => {
        setConfig(DEFAULT_CONFIG);
    };

    if (!isOpen) return null;

    const configurableWidgets = AVAILABLE_WIDGETS.filter(w => w.configurable && isWidgetVisible(w.id));

    const getWidgetName = (w: WidgetDefinition) => {
        const key = `widget_${w.id}`;
        const translated = t(key);
        return translated !== key ? translated : w.name;
    };

    const getWidgetDesc = (w: WidgetDefinition) => {
        const key = `widget_${w.id}_desc`;
        const translated = t(key);
        return translated !== key ? translated : w.description;
    };

    return (
        <div className="fixed inset-0 z-50 overflow-y-auto">
            <div className="flex min-h-full items-center justify-center p-4">
                <div className="fixed inset-0 bg-black/50 backdrop-blur-xs transition-opacity" />
                
                <div className="relative bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-2xl max-h-[85vh] overflow-hidden">
                    {/* Header */}
                    <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                        <div className="flex items-center space-x-3">
                            <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-lg">
                                <Settings2 className="w-5 h-5 text-primary-600 dark:text-primary-400" />
                            </div>
                            <div>
                                <h2 className="text-lg font-semibold text-gray-900 dark:text-white">{t('title')}</h2>
                                <p className="text-sm text-gray-500 dark:text-gray-400">{t('subtitle')}</p>
                            </div>
                        </div>
                        <button onClick={onClose} className="p-2 text-gray-400 hover:text-gray-500 dark:hover:text-gray-300">
                            <X className="w-5 h-5" />
                        </button>
                    </div>

                    {/* Tabs */}
                    <div className="flex border-b border-gray-200 dark:border-gray-700">
                        <button
                            onClick={() => setActiveTab('visibility')}
                            className={`flex-1 px-4 py-3 text-sm font-medium transition-colors ${
                                activeTab === 'visibility'
                                    ? 'text-primary-600 dark:text-primary-400 border-b-2 border-primary-600 dark:border-primary-400'
                                    : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
                            }`}
                        >
                            <Eye className="w-4 h-4 inline-block mr-2" />
                            {t('visibility')}
                        </button>
                        <button
                            onClick={() => setActiveTab('settings')}
                            className={`flex-1 px-4 py-3 text-sm font-medium transition-colors ${
                                activeTab === 'settings'
                                    ? 'text-primary-600 dark:text-primary-400 border-b-2 border-primary-600 dark:border-primary-400'
                                    : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
                            }`}
                        >
                            <Settings2 className="w-4 h-4 inline-block mr-2" />
                            {t('settings')}
                        </button>
                        <button
                            onClick={() => setActiveTab('add')}
                            className={`flex-1 px-4 py-3 text-sm font-medium transition-colors ${
                                activeTab === 'add'
                                    ? 'text-primary-600 dark:text-primary-400 border-b-2 border-primary-600 dark:border-primary-400'
                                    : 'text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300'
                            }`}
                        >
                            <Plus className="w-4 h-4 inline-block mr-2" />
                            {t('addWidgets')}
                        </button>
                    </div>

                    {/* Content */}
                    <div className="p-6 overflow-y-auto max-h-[50vh]">
                        {activeTab === 'visibility' && (
                            <div className="space-y-3">
                                <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
                                    {t('visibilityHint')}
                                </p>
                                {AVAILABLE_WIDGETS.map((widget) => {
                                    const Icon = widget.icon;
                                    const isVisible = isWidgetVisible(widget.id);
                                    return (
                                        <div
                                            key={widget.id}
                                            className={`flex items-center justify-between p-4 rounded-lg border transition-all ${
                                                isVisible
                                                    ? 'border-primary-200 bg-primary-50 dark:border-primary-800 dark:bg-primary-900/20'
                                                    : 'border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50 opacity-60'
                                            }`}
                                        >
                                            <div className="flex items-center space-x-3">
                                                <div className={`p-2 rounded-lg ${
                                                    isVisible
                                                        ? 'bg-primary-100 dark:bg-primary-900/50'
                                                        : 'bg-gray-200 dark:bg-gray-700'
                                                }`}>
                                                    <Icon className={`w-4 h-4 ${
                                                        isVisible
                                                            ? 'text-primary-600 dark:text-primary-400'
                                                            : 'text-gray-500 dark:text-gray-400'
                                                    }`} />
                                                </div>
                                                <div>
                                                    <p className="text-sm font-medium text-gray-900 dark:text-white">{getWidgetName(widget)}</p>
                                                    <p className="text-xs text-gray-500 dark:text-gray-400">{getWidgetDesc(widget)}</p>
                                                </div>
                                            </div>
                                            <button
                                                onClick={() => toggleWidgetVisibility(widget.id)}
                                                className={`p-2 rounded-lg transition-colors ${
                                                    isVisible
                                                        ? 'bg-primary-600 text-white hover:bg-primary-700'
                                                        : 'bg-gray-200 dark:bg-gray-700 text-gray-500 dark:text-gray-400 hover:bg-gray-300 dark:hover:bg-gray-600'
                                                }`}
                                            >
                                                {isVisible ? <Eye className="w-4 h-4" /> : <EyeOff className="w-4 h-4" />}
                                            </button>
                                        </div>
                                    );
                                })}
                            </div>
                        )}

                        {activeTab === 'settings' && (
                            <div className="space-y-6">
                                {configurableWidgets.length === 0 ? (
                                    <div className="text-center py-8 text-gray-500 dark:text-gray-400">
                                        <Settings2 className="w-12 h-12 mx-auto mb-3 opacity-50" />
                                        <p className="text-sm">{t('noConfigurable')}</p>
                                        <p className="text-xs mt-1">{t('enableInVisibility')}</p>
                                    </div>
                                ) : (
                                    configurableWidgets.map((widget) => {
                                        const Icon = widget.icon;
                                        const settings = config.widget_settings[widget.id] || widget.defaultConfig || {};
                                        
                                        return (
                                            <div key={widget.id} className="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                                                <div className="flex items-center space-x-3 mb-4">
                                                    <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-lg">
                                                        <Icon className="w-4 h-4 text-primary-600 dark:text-primary-400" />
                                                    </div>
                                                    <div>
                                                        <h3 className="text-sm font-medium text-gray-900 dark:text-white">{getWidgetName(widget)}</h3>
                                                        <p className="text-xs text-gray-500 dark:text-gray-400">{getWidgetDesc(widget)}</p>
                                                    </div>
                                                </div>
                                                
                                                {/* Widget-specific settings */}
                                                {widget.id === 'activity' && (
                                                    <div className="space-y-3">
                                                        <label className="block">
                                                            <span className="text-sm text-gray-600 dark:text-gray-400">{t('activityItemsCount')}</span>
                                                            <select
                                                                value={settings.max_items || 7}
                                                                onChange={(e) => updateWidgetSetting(widget.id, 'max_items', parseInt(e.target.value))}
                                                                className="mt-1 block w-full rounded-md border-gray-300 dark:border-gray-600 dark:bg-gray-700 dark:text-white shadow-sm focus:border-primary-500 focus:ring-primary-500 text-sm"
                                                            >
                                                                <option value={5}>{t('numItems', { count: 5 })}</option>
                                                                <option value={7}>{t('numItems', { count: 7 })}</option>
                                                                <option value={10}>{t('numItems', { count: 10 })}</option>
                                                                <option value={15}>{t('numItems', { count: 15 })}</option>
                                                            </select>
                                                        </label>
                                                    </div>
                                                )}
                                                
                                                {widget.id === 'activity-chart' && (
                                                    <div className="space-y-3">
                                                        <label className="block">
                                                            <span className="text-sm text-gray-600 dark:text-gray-400">{t('activityChartDays')}</span>
                                                            <select
                                                                value={settings.days || 7}
                                                                onChange={(e) => updateWidgetSetting(widget.id, 'days', parseInt(e.target.value))}
                                                                className="mt-1 block w-full rounded-md border-gray-300 dark:border-gray-600 dark:bg-gray-700 dark:text-white shadow-sm focus:border-primary-500 focus:ring-primary-500 text-sm"
                                                            >
                                                                <option value={7}>7 {t('days')}</option>
                                                                <option value={14}>14 {t('days')}</option>
                                                                <option value={30}>30 {t('days')}</option>
                                                            </select>
                                                        </label>
                                                    </div>
                                                )}

                                                {widget.id === 'storage-trends' && (
                                                    <div className="space-y-3">
                                                        <label className="block">
                                                            <span className="text-sm text-gray-600 dark:text-gray-400">{t('storageTrendsPeriod')}</span>
                                                            <select
                                                                value={settings.period || '30d'}
                                                                onChange={(e) => updateWidgetSetting(widget.id, 'period', e.target.value)}
                                                                className="mt-1 block w-full rounded-md border-gray-300 dark:border-gray-600 dark:bg-gray-700 dark:text-white shadow-sm focus:border-primary-500 focus:ring-primary-500 text-sm"
                                                            >
                                                                <option value="7d">7 {t('days')}</option>
                                                                <option value="30d">30 {t('days')}</option>
                                                                <option value="90d">90 {t('days')}</option>
                                                            </select>
                                                        </label>
                                                    </div>
                                                )}

                                                {widget.id === 'recent-uploads' && (
                                                    <div className="space-y-3">
                                                        <label className="block">
                                                            <span className="text-sm text-gray-600 dark:text-gray-400">{t('recentUploadsLimit')}</span>
                                                            <select
                                                                value={settings.limit || 5}
                                                                onChange={(e) => updateWidgetSetting(widget.id, 'limit', parseInt(e.target.value))}
                                                                className="mt-1 block w-full rounded-md border-gray-300 dark:border-gray-600 dark:bg-gray-700 dark:text-white shadow-sm focus:border-primary-500 focus:ring-primary-500 text-sm"
                                                            >
                                                                <option value={5}>5 {t('numItems', { count: 5 })}</option>
                                                                <option value={10}>10 {t('numItems', { count: 10 })}</option>
                                                                <option value={15}>15 {t('numItems', { count: 15 })}</option>
                                                            </select>
                                                        </label>
                                                    </div>
                                                )}

                                                {widget.id === 'upcoming-expiry' && (
                                                    <div className="space-y-3">
                                                        <label className="block">
                                                            <span className="text-sm text-gray-600 dark:text-gray-400">{t('upcomingExpiryDays')}</span>
                                                            <select
                                                                value={settings.days_ahead || 7}
                                                                onChange={(e) => updateWidgetSetting(widget.id, 'days_ahead', parseInt(e.target.value))}
                                                                className="mt-1 block w-full rounded-md border-gray-300 dark:border-gray-600 dark:bg-gray-700 dark:text-white shadow-sm focus:border-primary-500 focus:ring-primary-500 text-sm"
                                                            >
                                                                <option value={3}>3 {t('days')}</option>
                                                                <option value={7}>7 {t('days')}</option>
                                                                <option value={14}>14 {t('days')}</option>
                                                                <option value={30}>30 {t('days')}</option>
                                                            </select>
                                                        </label>
                                                    </div>
                                                )}

                                                {widget.id === 'notifications' && (
                                                    <div className="space-y-3">
                                                        <label className="block">
                                                            <span className="text-sm text-gray-600 dark:text-gray-400">{t('notificationsLimit')}</span>
                                                            <select
                                                                value={settings.limit || 5}
                                                                onChange={(e) => updateWidgetSetting(widget.id, 'limit', parseInt(e.target.value))}
                                                                className="mt-1 block w-full rounded-md border-gray-300 dark:border-gray-600 dark:bg-gray-700 dark:text-white shadow-sm focus:border-primary-500 focus:ring-primary-500 text-sm"
                                                            >
                                                                <option value={5}>5 {t('numItems', { count: 5 })}</option>
                                                                <option value={10}>10 {t('numItems', { count: 10 })}</option>
                                                                <option value={15}>15 {t('numItems', { count: 15 })}</option>
                                                            </select>
                                                        </label>
                                                    </div>
                                                )}

                                                {widget.id === 'requests' && (
                                                    <div className="space-y-3">
                                                        <label className="flex items-center cursor-pointer">
                                                            <input
                                                                type="checkbox"
                                                                checked={settings.show_expired || false}
                                                                onChange={(e) => updateWidgetSetting(widget.id, 'show_expired', e.target.checked)}
                                                                className="rounded border-gray-300 dark:border-gray-600 text-primary-600 focus:ring-primary-500"
                                                            />
                                                            <span className="ml-2 text-sm text-gray-600 dark:text-gray-400">{t('showExpiredRequests')}</span>
                                                        </label>
                                                    </div>
                                                )}
                                                
                                                {widget.id === 'departments' && (
                                                    <div className="space-y-3">
                                                        <label className="block">
                                                            <span className="text-sm text-gray-600 dark:text-gray-400">{t('maxDepartmentsToShow')}</span>
                                                            <select
                                                                value={settings.max_shown || 6}
                                                                onChange={(e) => updateWidgetSetting(widget.id, 'max_shown', parseInt(e.target.value))}
                                                                className="mt-1 block w-full rounded-md border-gray-300 dark:border-gray-600 dark:bg-gray-700 dark:text-white shadow-sm focus:border-primary-500 focus:ring-primary-500 text-sm"
                                                            >
                                                                <option value={4}>{t('numDepartments', { count: 4 })}</option>
                                                                <option value={6}>{t('numDepartments', { count: 6 })}</option>
                                                                <option value={8}>{t('numDepartments', { count: 8 })}</option>
                                                                <option value={12}>{t('numDepartments', { count: 12 })}</option>
                                                            </select>
                                                        </label>
                                                    </div>
                                                )}
                                            </div>
                                        );
                                    })
                                )}
                            </div>
                        )}

                        {activeTab === 'add' && (
                            <div className="space-y-4">
                                <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
                                    {t('addWidgetsHint')}
                                </p>
                                
                                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                                    {ADDITIONAL_WIDGETS.map((widget) => {
                                        const Icon = widget.icon;
                                        const isAdded = isWidgetVisible(widget.id) || config.custom_widgets.includes(widget.id);
                                        
                                        return (
                                            <div
                                                key={widget.id}
                                                className={`flex items-start p-4 rounded-lg border transition-all ${
                                                    isAdded
                                                        ? 'border-green-200 bg-green-50 dark:border-green-800 dark:bg-green-900/20'
                                                        : 'border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 hover:border-primary-300 dark:hover:border-primary-700'
                                                }`}
                                            >
                                                <div className={`p-2 rounded-lg mr-3 ${
                                                    isAdded
                                                        ? 'bg-green-100 dark:bg-green-900/50'
                                                        : 'bg-gray-100 dark:bg-gray-700'
                                                }`}>
                                                    <Icon className={`w-5 h-5 ${
                                                        isAdded
                                                            ? 'text-green-600 dark:text-green-400'
                                                            : 'text-gray-500 dark:text-gray-400'
                                                    }`} />
                                                </div>
                                                <div className="flex-1 min-w-0">
                                                    <div className="flex items-center justify-between">
                                                        <p className="text-sm font-medium text-gray-900 dark:text-white">{getWidgetName(widget)}</p>
                                                        {isAdded ? (
                                                            <span className="text-xs text-green-600 dark:text-green-400 font-medium">{t('added')}</span>
                                                        ) : (
                                                            <button
                                                                onClick={() => {
                                                                    setConfig(prev => ({
                                                                        ...prev,
                                                                        custom_widgets: [...prev.custom_widgets, widget.id],
                                                                        visible_widgets: [...prev.visible_widgets, widget.id]
                                                                    }));
                                                                }}
                                                                className="text-xs text-primary-600 dark:text-primary-400 font-medium hover:text-primary-700 dark:hover:text-primary-300 flex items-center"
                                                            >
                                                                <Plus className="w-3 h-3 mr-1" />
                                                                {t('add')}
                                                            </button>
                                                        )}
                                                    </div>
                                                    <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">{getWidgetDesc(widget)}</p>
                                                    {widget.configurable && (
                                                        <span className="inline-flex items-center mt-2 px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400">
                                                            <Settings2 className="w-3 h-3 mr-1" />
                                                            {t('configurable')}
                                                        </span>
                                                    )}
                                                </div>
                                            </div>
                                        );
                                    })}
                                </div>

                                <div className="mt-6 p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-100 dark:border-blue-800">
                                    <p className="text-sm text-blue-800 dark:text-blue-300">
                                        {t('comingSoonNote')}
                                    </p>
                                </div>
                            </div>
                        )}
                    </div>

                    {/* Footer */}
                    <div className="flex items-center justify-between px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
                        <button
                            onClick={handleReset}
                            className="px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200"
                        >
                            {t('resetDefault')}
                        </button>
                        <div className="flex space-x-3">
                            <button
                                onClick={onClose}
                                className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-600"
                            >
                                {tCommon('cancel')}
                            </button>
                            <button
                                onClick={handleSave}
                                disabled={isSaving}
                                className="px-4 py-2 text-sm font-medium text-white bg-primary-600 rounded-lg hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed"
                            >
                                {isSaving ? tCommon('saving') : t('saveChanges')}
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    );
}
