import { useState, useEffect } from 'react';
import { Bell, AlertCircle, CheckCircle, Info, AlertTriangle } from 'lucide-react';
import { useAuthFetch } from '../../context/AuthContext';
import { formatDistanceToNow } from 'date-fns';

interface Notification {
    id: string;
    type: 'info' | 'success' | 'warning' | 'error';
    title: string;
    message: string;
    timestamp: string;
}

interface NotificationsWidgetProps {
    limit?: number;
}

export function NotificationsWidget({ limit = 5 }: NotificationsWidgetProps) {
    const [notifications, setNotifications] = useState<Notification[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const authFetch = useAuthFetch();

    useEffect(() => {
        fetchNotifications();
    }, [limit]);

    const fetchNotifications = async () => {
        try {
            // Fetch recent activity logs and transform into notifications
            const res = await authFetch('/api/activity-logs?limit=' + limit);
            if (res.ok) {
                const data = await res.json();
                const notifs: Notification[] = (data.logs || []).slice(0, limit).map((log: any) => {
                    let type: 'info' | 'success' | 'warning' | 'error' = 'info';
                    let title = log.action || 'Activity';
                    
                    if (log.action?.includes('error') || log.action?.includes('failed')) {
                        type = 'error';
                        title = 'Error';
                    } else if (log.action?.includes('warning') || log.action?.includes('alert')) {
                        type = 'warning';
                        title = 'Warning';
                    } else if (log.action?.includes('success') || log.action?.includes('created') || log.action?.includes('completed')) {
                        type = 'success';
                        title = 'Success';
                    }
                    
                    return {
                        id: log.id,
                        type,
                        title: title.charAt(0).toUpperCase() + title.slice(1).replace(/_/g, ' '),
                        message: `${log.user || 'System'}: ${log.action?.replace(/_/g, ' ')} ${log.resource || ''}`.trim(),
                        timestamp: log.timestamp
                    };
                });
                setNotifications(notifs);
            }
        } catch (error) {
            console.error('Failed to fetch notifications', error);
        } finally {
            setIsLoading(false);
        }
    };

    const getIcon = (type: string) => {
        switch (type) {
            case 'error':
                return <AlertCircle className="w-4 h-4 text-red-500" />;
            case 'warning':
                return <AlertTriangle className="w-4 h-4 text-orange-500" />;
            case 'success':
                return <CheckCircle className="w-4 h-4 text-green-500" />;
            default:
                return <Info className="w-4 h-4 text-blue-500" />;
        }
    };

    const getBgColor = (type: string) => {
        switch (type) {
            case 'error':
                return 'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800';
            case 'warning':
                return 'bg-orange-50 dark:bg-orange-900/20 border-orange-200 dark:border-orange-800';
            case 'success':
                return 'bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-800';
            default:
                return 'bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800';
        }
    };

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-6 h-full">
            <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-medium text-gray-900 dark:text-white">Notifications</h3>
                <Bell className="w-5 h-5 text-gray-400" />
            </div>
            
            {isLoading ? (
                <div className="space-y-3">
                    {[...Array(3)].map((_, i) => (
                        <div key={i} className="h-16 bg-gray-200 dark:bg-gray-700 rounded-lg animate-pulse"></div>
                    ))}
                </div>
            ) : notifications.length === 0 ? (
                <div className="text-center py-6 text-gray-500 dark:text-gray-400">
                    <Bell className="w-8 h-8 mx-auto mb-2 opacity-50" />
                    <p className="text-sm">No notifications</p>
                </div>
            ) : (
                <div className="space-y-3">
                    {notifications.map((notif) => (
                        <div key={notif.id} className={`p-3 rounded-lg border ${getBgColor(notif.type)}`}>
                            <div className="flex items-start">
                                <div className="flex-shrink-0 mt-0.5">
                                    {getIcon(notif.type)}
                                </div>
                                <div className="ml-3 flex-1 min-w-0">
                                    <p className="text-sm font-medium text-gray-900 dark:text-white truncate">
                                        {notif.title}
                                    </p>
                                    <p className="text-xs text-gray-600 dark:text-gray-400 truncate">
                                        {notif.message}
                                    </p>
                                    <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
                                        {formatDistanceToNow(new Date(notif.timestamp), { addSuffix: true })}
                                    </p>
                                </div>
                            </div>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
