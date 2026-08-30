import { useState, useEffect } from 'react';
import { Bell, Check, CheckCheck, Trash2, AlertCircle, Upload, Clock, UserPlus, Shield, HardDrive, Share, Settings, Filter, Mail, BellRing } from 'lucide-react';
import { useAuthFetch, useAuth } from '../context/AuthContext';
import { formatDistanceToNow } from 'date-fns';

interface Notification {
    id: string;
    notification_type: string;
    title: string;
    message: string;
    metadata: Record<string, any>;
    is_read: boolean;
    created_at: string;
}

interface NotificationListResponse {
    notifications: Notification[];
    total: number;
    unread_count: number;
    page: number;
    limit: number;
}

interface NotificationPreference {
    id: string;
    user_id: string;
    event_type: string;
    email_enabled: boolean;
    in_app_enabled: boolean;
}

interface PreferenceLabel {
    event_type: string;
    label: string;
    description: string;
}

export function Notifications() {
    const { user } = useAuth();
    const authFetch = useAuthFetch();
    const [activeTab, setActiveTab] = useState<'all' | 'unread' | 'preferences'>('all');
    const [notifications, setNotifications] = useState<Notification[]>([]);
    const [unreadCount, setUnreadCount] = useState(0);
    const [total, setTotal] = useState(0);
    const [page, setPage] = useState(1);
    const [isLoading, setIsLoading] = useState(true);
    const [preferences, setPreferences] = useState<NotificationPreference[]>([]);
    const [preferenceLabels, setPreferenceLabels] = useState<PreferenceLabel[]>([]);
    const [savingPrefs, setSavingPrefs] = useState(false);

    useEffect(() => {
        if (activeTab === 'preferences') {
            fetchPreferences();
            fetchPreferenceLabels();
        } else {
            fetchNotifications();
        }
    }, [activeTab, page]);

    const fetchNotifications = async () => {
        setIsLoading(true);
        try {
            const unreadOnly = activeTab === 'unread';
            const res = await authFetch(`/api/notifications?page=${page}&limit=20&unread_only=${unreadOnly}`);
            if (res.ok) {
                const data: NotificationListResponse = await res.json();
                setNotifications(data.notifications || []);
                setUnreadCount(data.unread_count || 0);
                setTotal(data.total || 0);
            }
        } catch (error) {
            console.error('Failed to fetch notifications', error);
        } finally {
            setIsLoading(false);
        }
    };

    const fetchPreferences = async () => {
        setIsLoading(true);
        try {
            const res = await authFetch('/api/notifications/preferences');
            if (res.ok) {
                const data = await res.json();
                setPreferences(data);
            }
        } catch (error) {
            console.error('Failed to fetch preferences', error);
        } finally {
            setIsLoading(false);
        }
    };

    const fetchPreferenceLabels = async () => {
        try {
            const res = await authFetch('/api/notifications/preference-labels');
            if (res.ok) {
                const data = await res.json();
                setPreferenceLabels(data);
            }
        } catch (error) {
            console.error('Failed to fetch preference labels', error);
        }
    };

    const markAsRead = async (id: string) => {
        try {
            await authFetch(`/api/notifications/${id}/read`, { method: 'PUT' });
            setNotifications(prev => 
                prev.map(n => n.id === id ? { ...n, is_read: true } : n)
            );
            setUnreadCount(prev => Math.max(0, prev - 1));
        } catch (error) {
            console.error('Failed to mark as read', error);
        }
    };

    const markAllAsRead = async () => {
        try {
            await authFetch('/api/notifications/read-all', { method: 'PUT' });
            setNotifications(prev => prev.map(n => ({ ...n, is_read: true })));
            setUnreadCount(0);
        } catch (error) {
            console.error('Failed to mark all as read', error);
        }
    };

    const deleteNotification = async (id: string) => {
        try {
            await authFetch(`/api/notifications/${id}`, { method: 'DELETE' });
            const deletedNotif = notifications.find(n => n.id === id);
            setNotifications(prev => prev.filter(n => n.id !== id));
            if (deletedNotif && !deletedNotif.is_read) {
                setUnreadCount(prev => Math.max(0, prev - 1));
            }
            setTotal(prev => prev - 1);
        } catch (error) {
            console.error('Failed to delete notification', error);
        }
    };

    const updatePreference = async (eventType: string, field: 'email_enabled' | 'in_app_enabled', value: boolean) => {
        setSavingPrefs(true);
        try {
            const res = await authFetch('/api/notifications/preferences', {
                method: 'PUT',
                body: JSON.stringify({
                    preferences: [{
                        event_type: eventType,
                        [field]: value
                    }]
                })
            });
            if (res.ok) {
                const updated = await res.json();
                setPreferences(updated);
            }
        } catch (error) {
            console.error('Failed to update preference', error);
        } finally {
            setSavingPrefs(false);
        }
    };

    const getNotificationIcon = (type: string) => {
        switch (type) {
            case 'file_upload':
                return <Upload className="w-5 h-5 text-blue-500" />;
            case 'request_expiring':
                return <Clock className="w-5 h-5 text-orange-500" />;
            case 'user_created':
            case 'role_changed':
                return <UserPlus className="w-5 h-5 text-green-500" />;
            case 'compliance_alert':
                return <Shield className="w-5 h-5 text-purple-500" />;
            case 'storage_warning':
                return <HardDrive className="w-5 h-5 text-red-500" />;
            case 'file_shared':
                return <Share className="w-5 h-5 text-teal-500" />;
            default:
                return <AlertCircle className="w-5 h-5 text-gray-500" />;
        }
    };

    const canAccessPreference = (eventType: string) => {
        // Only admins can configure admin-only notification types
        const adminOnlyTypes = ['user_action', 'compliance_alert', 'storage_warning'];
        if (adminOnlyTypes.includes(eventType)) {
            return user?.role === 'SuperAdmin' || user?.role === 'Admin';
        }
        return true;
    };

    const totalPages = Math.ceil(total / 20);

    return (
        <div className="space-y-6">
            <div className="flex items-center justify-between">
                <div>
                    <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Notifications</h1>
                    <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                        Manage your notifications and preferences
                    </p>
                </div>
                {activeTab !== 'preferences' && unreadCount > 0 && (
                    <button
                        onClick={markAllAsRead}
                        className="inline-flex items-center px-4 py-2 text-sm font-medium text-primary-600 dark:text-primary-400 bg-primary-50 dark:bg-primary-900/20 rounded-lg hover:bg-primary-100 dark:hover:bg-primary-900/30 transition-colors"
                    >
                        <CheckCheck className="w-4 h-4 mr-2" />
                        Mark all as read
                    </button>
                )}
            </div>

            {/* Tabs */}
            <div className="border-b border-gray-200 dark:border-gray-700">
                <nav className="flex space-x-8">
                    <button
                        onClick={() => { setActiveTab('all'); setPage(1); }}
                        className={`pb-4 px-1 text-sm font-medium border-b-2 transition-colors ${
                            activeTab === 'all'
                                ? 'border-primary-500 text-primary-600 dark:text-primary-400'
                                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                        }`}
                    >
                        <Bell className="w-4 h-4 inline-block mr-2" />
                        All Notifications
                    </button>
                    <button
                        onClick={() => { setActiveTab('unread'); setPage(1); }}
                        className={`pb-4 px-1 text-sm font-medium border-b-2 transition-colors ${
                            activeTab === 'unread'
                                ? 'border-primary-500 text-primary-600 dark:text-primary-400'
                                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                        }`}
                    >
                        <Filter className="w-4 h-4 inline-block mr-2" />
                        Unread
                        {unreadCount > 0 && (
                            <span className="ml-2 px-2 py-0.5 text-xs font-medium bg-red-100 text-red-600 dark:bg-red-900/30 dark:text-red-400 rounded-full">
                                {unreadCount}
                            </span>
                        )}
                    </button>
                    <button
                        onClick={() => setActiveTab('preferences')}
                        className={`pb-4 px-1 text-sm font-medium border-b-2 transition-colors ${
                            activeTab === 'preferences'
                                ? 'border-primary-500 text-primary-600 dark:text-primary-400'
                                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
                        }`}
                    >
                        <Settings className="w-4 h-4 inline-block mr-2" />
                        Preferences
                    </button>
                </nav>
            </div>

            {/* Content */}
            {activeTab === 'preferences' ? (
                <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700">
                    <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Notification Preferences</h2>
                        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                            Choose how you want to be notified about different events
                        </p>
                    </div>
                    
                    {isLoading ? (
                        <div className="p-6 space-y-4">
                            {[...Array(4)].map((_, i) => (
                                <div key={i} className="animate-pulse flex items-center justify-between">
                                    <div className="space-y-2">
                                        <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-32"></div>
                                        <div className="h-3 bg-gray-200 dark:bg-gray-700 rounded w-48"></div>
                                    </div>
                                    <div className="flex space-x-4">
                                        <div className="h-6 w-12 bg-gray-200 dark:bg-gray-700 rounded"></div>
                                        <div className="h-6 w-12 bg-gray-200 dark:bg-gray-700 rounded"></div>
                                    </div>
                                </div>
                            ))}
                        </div>
                    ) : (
                        <div className="divide-y divide-gray-200 dark:divide-gray-700">
                            {preferenceLabels.map((label) => {
                                const pref = preferences.find(p => p.event_type === label.event_type);
                                const canAccess = canAccessPreference(label.event_type);
                                
                                if (!canAccess) return null;
                                
                                return (
                                    <div key={label.event_type} className="p-6 flex items-center justify-between">
                                        <div className="flex-1">
                                            <h3 className="text-sm font-medium text-gray-900 dark:text-white">{label.label}</h3>
                                            <p className="text-sm text-gray-500 dark:text-gray-400">{label.description}</p>
                                        </div>
                                        <div className="flex items-center space-x-6">
                                            <label className="flex items-center space-x-2 cursor-pointer">
                                                <input
                                                    type="checkbox"
                                                    checked={pref?.in_app_enabled ?? true}
                                                    onChange={(e) => updatePreference(label.event_type, 'in_app_enabled', e.target.checked)}
                                                    disabled={savingPrefs}
                                                    className="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                                                />
                                                <BellRing className="w-4 h-4 text-gray-400" />
                                                <span className="text-sm text-gray-600 dark:text-gray-300">In-app</span>
                                            </label>
                                            <label className="flex items-center space-x-2 cursor-pointer">
                                                <input
                                                    type="checkbox"
                                                    checked={pref?.email_enabled ?? true}
                                                    onChange={(e) => updatePreference(label.event_type, 'email_enabled', e.target.checked)}
                                                    disabled={savingPrefs}
                                                    className="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                                                />
                                                <Mail className="w-4 h-4 text-gray-400" />
                                                <span className="text-sm text-gray-600 dark:text-gray-300">Email</span>
                                            </label>
                                        </div>
                                    </div>
                                );
                            })}
                        </div>
                    )}
                </div>
            ) : (
                <>
                    {isLoading ? (
                        <div className="space-y-4">
                            {[...Array(5)].map((_, i) => (
                                <div key={i} className="bg-white dark:bg-gray-800 rounded-lg p-4 animate-pulse flex space-x-4">
                                    <div className="w-10 h-10 bg-gray-200 dark:bg-gray-700 rounded-full"></div>
                                    <div className="flex-1 space-y-2">
                                        <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/4"></div>
                                        <div className="h-3 bg-gray-200 dark:bg-gray-700 rounded w-3/4"></div>
                                        <div className="h-3 bg-gray-200 dark:bg-gray-700 rounded w-1/2"></div>
                                    </div>
                                </div>
                            ))}
                        </div>
                    ) : notifications.length === 0 ? (
                        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-12 text-center">
                            <Bell className="w-16 h-16 mx-auto text-gray-300 dark:text-gray-600 mb-4" />
                            <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-2">
                                {activeTab === 'unread' ? 'No unread notifications' : 'No notifications yet'}
                            </h3>
                            <p className="text-gray-500 dark:text-gray-400">
                                {activeTab === 'unread' 
                                    ? 'You\'re all caught up!' 
                                    : 'Notifications about important events will appear here.'}
                            </p>
                        </div>
                    ) : (
                        <div className="space-y-4">
                            {notifications.map((notification) => (
                                <div
                                    key={notification.id}
                                    className={`bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-4 transition-colors ${
                                        !notification.is_read ? 'border-l-4 border-l-primary-500' : ''
                                    }`}
                                >
                                    <div className="flex items-start space-x-4">
                                        <div className="flex-shrink-0 p-2 bg-gray-100 dark:bg-gray-700 rounded-full">
                                            {getNotificationIcon(notification.notification_type)}
                                        </div>
                                        <div className="flex-1 min-w-0">
                                            <div className="flex items-start justify-between">
                                                <div>
                                                    <h3 className={`text-sm font-medium ${
                                                        notification.is_read 
                                                            ? 'text-gray-700 dark:text-gray-300' 
                                                            : 'text-gray-900 dark:text-white'
                                                    }`}>
                                                        {notification.title}
                                                    </h3>
                                                    <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                                                        {notification.message}
                                                    </p>
                                                    <p className="text-xs text-gray-400 dark:text-gray-500 mt-2">
                                                        {formatDistanceToNow(new Date(notification.created_at), { addSuffix: true })}
                                                    </p>
                                                </div>
                                                <div className="flex items-center space-x-2 ml-4">
                                                    {!notification.is_read && (
                                                        <button
                                                            onClick={() => markAsRead(notification.id)}
                                                            className="p-2 text-gray-400 hover:text-green-500 hover:bg-green-50 dark:hover:bg-green-900/20 rounded-lg transition-colors"
                                                            title="Mark as read"
                                                        >
                                                            <Check className="w-4 h-4" />
                                                        </button>
                                                    )}
                                                    <button
                                                        onClick={() => deleteNotification(notification.id)}
                                                        className="p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg transition-colors"
                                                        title="Delete"
                                                    >
                                                        <Trash2 className="w-4 h-4" />
                                                    </button>
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}

                    {/* Pagination */}
                    {totalPages > 1 && (
                        <div className="flex items-center justify-between pt-4">
                            <p className="text-sm text-gray-500 dark:text-gray-400">
                                Showing {((page - 1) * 20) + 1} to {Math.min(page * 20, total)} of {total} notifications
                            </p>
                            <div className="flex items-center space-x-2">
                                <button
                                    onClick={() => setPage(prev => Math.max(1, prev - 1))}
                                    disabled={page === 1}
                                    className="px-3 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
                                >
                                    Previous
                                </button>
                                <span className="text-sm text-gray-500 dark:text-gray-400">
                                    Page {page} of {totalPages}
                                </span>
                                <button
                                    onClick={() => setPage(prev => Math.min(totalPages, prev + 1))}
                                    disabled={page === totalPages}
                                    className="px-3 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed"
                                >
                                    Next
                                </button>
                            </div>
                        </div>
                    )}
                </>
            )}
        </div>
    );
}
