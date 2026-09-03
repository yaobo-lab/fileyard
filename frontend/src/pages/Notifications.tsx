import { useState, useEffect } from 'react';
import { Bell, Check, CheckCheck, Trash2, AlertCircle, Upload, Clock, UserPlus, Shield, HardDrive, Share, Settings, Filter, Mail, BellRing } from 'lucide-react';
import { useAuthFetch, useAuth } from '../context/AuthContext';
import { formatDistanceToNow } from 'date-fns';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Switch } from '@/components/ui/switch';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import clsx from 'clsx';

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
                    <h1 className="text-2xl font-bold tracking-tight text-foreground">Notifications</h1>
                    <p className="text-sm text-muted-foreground mt-1">
                        Manage your notifications, alerts and preferences
                    </p>
                </div>
                {activeTab !== 'preferences' && unreadCount > 0 && (
                    <Button
                        variant="outline"
                        size="sm"
                        onClick={markAllAsRead}
                        className="gap-1.5 h-9"
                    >
                        <CheckCheck className="w-4 h-4 text-primary" />
                        <span>Mark all as read</span>
                    </Button>
                )}
            </div>

            {/* Tabs */}
            <div className="border-b border-border">
                <nav className="flex space-x-6">
                    <button
                        onClick={() => { setActiveTab('all'); setPage(1); }}
                        className={clsx(
                            "pb-3 text-sm font-medium border-b-2 transition-all flex items-center gap-2 cursor-pointer",
                            activeTab === 'all'
                                ? 'border-primary text-foreground font-semibold'
                                : 'border-transparent text-muted-foreground hover:text-foreground'
                        )}
                    >
                        <Bell className="w-4 h-4" />
                        <span>All Notifications</span>
                    </button>
                    <button
                        onClick={() => { setActiveTab('unread'); setPage(1); }}
                        className={clsx(
                            "pb-3 text-sm font-medium border-b-2 transition-all flex items-center gap-2 cursor-pointer",
                            activeTab === 'unread'
                                ? 'border-primary text-foreground font-semibold'
                                : 'border-transparent text-muted-foreground hover:text-foreground'
                        )}
                    >
                        <Filter className="w-4 h-4" />
                        <span>Unread</span>
                        {unreadCount > 0 && (
                            <Badge variant="destructive" className="ml-1 px-1.5 py-0 text-[11px] h-4">
                                {unreadCount}
                            </Badge>
                        )}
                    </button>
                    <button
                        onClick={() => setActiveTab('preferences')}
                        className={clsx(
                            "pb-3 text-sm font-medium border-b-2 transition-all flex items-center gap-2 cursor-pointer",
                            activeTab === 'preferences'
                                ? 'border-primary text-foreground font-semibold'
                                : 'border-transparent text-muted-foreground hover:text-foreground'
                        )}
                    >
                        <Settings className="w-4 h-4" />
                        <span>Preferences</span>
                    </button>
                </nav>
            </div>

            {/* Content */}
            {activeTab === 'preferences' ? (
                <Card className="shadow-xs">
                    <CardHeader className="pb-4">
                        <CardTitle className="text-lg">Notification Preferences</CardTitle>
                        <CardDescription>
                            Choose how you want to be notified about different workspace events
                        </CardDescription>
                    </CardHeader>
                    
                    <CardContent className="p-0">
                        {isLoading ? (
                            <div className="p-6 space-y-4">
                                {[...Array(4)].map((_, i) => (
                                    <div key={i} className="animate-pulse flex items-center justify-between">
                                        <div className="space-y-2">
                                            <div className="h-4 bg-muted rounded w-32"></div>
                                            <div className="h-3 bg-muted rounded w-48"></div>
                                        </div>
                                        <div className="flex space-x-4">
                                            <div className="h-6 w-12 bg-muted rounded"></div>
                                            <div className="h-6 w-12 bg-muted rounded"></div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                        ) : (
                            <div className="divide-y divide-border">
                                {preferenceLabels.map((label) => {
                                    const pref = preferences.find(p => p.event_type === label.event_type);
                                    const canAccess = canAccessPreference(label.event_type);
                                    
                                    if (!canAccess) return null;
                                    
                                    return (
                                        <div key={label.event_type} className="p-5 flex items-center justify-between gap-4">
                                            <div className="flex-1">
                                                <h3 className="text-sm font-medium text-foreground">{label.label}</h3>
                                                <p className="text-xs text-muted-foreground mt-0.5">{label.description}</p>
                                            </div>
                                            <div className="flex items-center space-x-6 shrink-0">
                                                <div className="flex items-center space-x-2">
                                                    <Switch
                                                        checked={pref?.in_app_enabled ?? true}
                                                        onCheckedChange={(checked) => updatePreference(label.event_type, 'in_app_enabled', checked)}
                                                        disabled={savingPrefs}
                                                    />
                                                    <BellRing className="w-4 h-4 text-muted-foreground" />
                                                    <span className="text-xs text-muted-foreground">In-app</span>
                                                </div>
                                                <div className="flex items-center space-x-2">
                                                    <Switch
                                                        checked={pref?.email_enabled ?? true}
                                                        onCheckedChange={(checked) => updatePreference(label.event_type, 'email_enabled', checked)}
                                                        disabled={savingPrefs}
                                                    />
                                                    <Mail className="w-4 h-4 text-muted-foreground" />
                                                    <span className="text-xs text-muted-foreground">Email</span>
                                                </div>
                                            </div>
                                        </div>
                                    );
                                })}
                            </div>
                        )}
                    </CardContent>
                </Card>
            ) : (
                <>
                    {isLoading ? (
                        <div className="space-y-3">
                            {[...Array(5)].map((_, i) => (
                                <div key={i} className="bg-card rounded-xl p-4 border border-border animate-pulse flex space-x-4">
                                    <div className="w-9 h-9 bg-muted rounded-full"></div>
                                    <div className="flex-1 space-y-2">
                                        <div className="h-4 bg-muted rounded w-1/4"></div>
                                        <div className="h-3 bg-muted rounded w-3/4"></div>
                                        <div className="h-3 bg-muted rounded w-1/2"></div>
                                    </div>
                                </div>
                            ))}
                        </div>
                    ) : notifications.length === 0 ? (
                        <Card className="p-12 text-center shadow-xs">
                            <Bell className="w-12 h-12 mx-auto text-muted-foreground/40 mb-3" />
                            <h3 className="text-base font-semibold text-foreground mb-1">
                                {activeTab === 'unread' ? 'No unread notifications' : 'No notifications yet'}
                            </h3>
                            <p className="text-xs text-muted-foreground">
                                {activeTab === 'unread' 
                                    ? 'You are all caught up!' 
                                    : 'Notifications about important events will appear here.'}
                            </p>
                        </Card>
                    ) : (
                        <div className="space-y-2.5">
                            {notifications.map((notification) => (
                                <div
                                    key={notification.id}
                                    className={clsx(
                                        "bg-card rounded-xl border p-4 transition-all shadow-xs",
                                        !notification.is_read ? 'border-l-4 border-l-primary border-border bg-muted/10' : 'border-border'
                                    )}
                                >
                                    <div className="flex items-start space-x-3.5">
                                        <div className="flex-shrink-0 p-2 bg-muted rounded-full text-foreground mt-0.5">
                                            {getNotificationIcon(notification.notification_type)}
                                        </div>
                                        <div className="flex-1 min-w-0">
                                            <div className="flex items-start justify-between gap-3">
                                                <div>
                                                    <h3 className={clsx(
                                                        "text-sm font-medium",
                                                        notification.is_read ? 'text-muted-foreground' : 'text-foreground font-semibold'
                                                    )}>
                                                        {notification.title}
                                                    </h3>
                                                    <p className="text-xs text-muted-foreground mt-1 leading-relaxed">
                                                        {notification.message}
                                                    </p>
                                                    <p className="text-[11px] text-muted-foreground/60 mt-1.5">
                                                        {formatDistanceToNow(new Date(notification.created_at), { addSuffix: true })}
                                                    </p>
                                                </div>
                                                <div className="flex items-center space-x-1 ml-2 shrink-0">
                                                    {!notification.is_read && (
                                                        <Button
                                                            variant="ghost"
                                                            size="icon"
                                                            onClick={() => markAsRead(notification.id)}
                                                            className="h-8 w-8 text-muted-foreground hover:text-emerald-500"
                                                            title="Mark as read"
                                                        >
                                                            <Check className="w-4 h-4" />
                                                        </Button>
                                                    )}
                                                    <Button
                                                        variant="ghost"
                                                        size="icon"
                                                        onClick={() => deleteNotification(notification.id)}
                                                        className="h-8 w-8 text-muted-foreground hover:text-destructive"
                                                        title="Delete"
                                                    >
                                                        <Trash2 className="w-4 h-4" />
                                                    </Button>
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
                            <p className="text-xs text-muted-foreground">
                                Showing {((page - 1) * 20) + 1} to {Math.min(page * 20, total)} of {total} notifications
                            </p>
                            <div className="flex items-center space-x-2">
                                <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={() => setPage(prev => Math.max(1, prev - 1))}
                                    disabled={page === 1}
                                    className="h-8"
                                >
                                    Previous
                                </Button>
                                <span className="text-xs text-muted-foreground">
                                    Page {page} of {totalPages}
                                </span>
                                <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={() => setPage(prev => Math.min(totalPages, prev + 1))}
                                    disabled={page === totalPages}
                                    className="h-8"
                                >
                                    Next
                                </Button>
                            </div>
                        </div>
                    )}
                </>
            )}
        </div>
    );
}
