import { useState, useEffect } from 'react';
import { Calendar, AlertTriangle, FileText, Link as LinkIcon } from 'lucide-react';
import { useAuthFetch } from '../../context/AuthContext';
import { formatDistanceToNow } from 'date-fns';

interface ExpiringItem {
    id: string;
    name: string;
    type: 'file_request' | 'file';
    expires_at: string;
    days_until: number;
}

interface ExpiringWidgetProps {
    daysAhead?: number;
}

export function ExpiringWidget({ daysAhead = 7 }: ExpiringWidgetProps) {
    const [items, setItems] = useState<ExpiringItem[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const authFetch = useAuthFetch();

    useEffect(() => {
        fetchExpiringItems();
    }, [daysAhead]);

    const fetchExpiringItems = async () => {
        try {
            // Fetch expiring file requests
            const res = await authFetch('/api/file-requests?status=active');
            if (res.ok) {
                const requests = await res.json();
                const now = new Date();
                const cutoff = new Date();
                cutoff.setDate(cutoff.getDate() + daysAhead);
                
                const expiringItems: ExpiringItem[] = (requests || [])
                    .filter((r: any) => {
                        const expiry = new Date(r.expires_at);
                        return expiry > now && expiry <= cutoff;
                    })
                    .map((r: any) => ({
                        id: r.id,
                        name: r.name,
                        type: 'file_request' as const,
                        expires_at: r.expires_at,
                        days_until: Math.ceil((new Date(r.expires_at).getTime() - now.getTime()) / (1000 * 60 * 60 * 24))
                    }))
                    .sort((a: ExpiringItem, b: ExpiringItem) => a.days_until - b.days_until);
                
                setItems(expiringItems.slice(0, 5));
            }
        } catch (error) {
            console.error('Failed to fetch expiring items', error);
        } finally {
            setIsLoading(false);
        }
    };

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-6 h-full">
            <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-medium text-gray-900 dark:text-white">Expiring Soon</h3>
                <Calendar className="w-5 h-5 text-gray-400" />
            </div>
            
            {isLoading ? (
                <div className="animate-pulse space-y-3">
                    {[...Array(3)].map((_, i) => (
                        <div key={i} className="h-12 bg-gray-200 dark:bg-gray-700 rounded"></div>
                    ))}
                </div>
            ) : items.length === 0 ? (
                <div className="text-center py-6 text-gray-500 dark:text-gray-400">
                    <Calendar className="w-8 h-8 mx-auto mb-2 opacity-50" />
                    <p className="text-sm">Nothing expiring in the next {daysAhead} days</p>
                </div>
            ) : (
                <div className="space-y-3">
                    {items.map((item) => (
                        <div 
                            key={item.id} 
                            className={`flex items-center p-3 rounded-lg ${
                                item.days_until <= 1 
                                    ? 'bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800'
                                    : item.days_until <= 3
                                    ? 'bg-orange-50 dark:bg-orange-900/20 border border-orange-200 dark:border-orange-800'
                                    : 'bg-gray-50 dark:bg-gray-700/50'
                            }`}
                        >
                            <div className={`p-2 rounded-lg mr-3 ${
                                item.days_until <= 1 
                                    ? 'bg-red-100 dark:bg-red-900/50'
                                    : item.days_until <= 3
                                    ? 'bg-orange-100 dark:bg-orange-900/50'
                                    : 'bg-gray-200 dark:bg-gray-600'
                            }`}>
                                {item.type === 'file_request' ? (
                                    <LinkIcon className={`w-4 h-4 ${
                                        item.days_until <= 1 ? 'text-red-600 dark:text-red-400' :
                                        item.days_until <= 3 ? 'text-orange-600 dark:text-orange-400' :
                                        'text-gray-600 dark:text-gray-400'
                                    }`} />
                                ) : (
                                    <FileText className={`w-4 h-4 ${
                                        item.days_until <= 1 ? 'text-red-600 dark:text-red-400' :
                                        item.days_until <= 3 ? 'text-orange-600 dark:text-orange-400' :
                                        'text-gray-600 dark:text-gray-400'
                                    }`} />
                                )}
                            </div>
                            <div className="flex-1 min-w-0">
                                <p className="text-sm font-medium text-gray-900 dark:text-white truncate">{item.name}</p>
                                <p className={`text-xs ${
                                    item.days_until <= 1 ? 'text-red-600 dark:text-red-400' :
                                    item.days_until <= 3 ? 'text-orange-600 dark:text-orange-400' :
                                    'text-gray-500 dark:text-gray-400'
                                }`}>
                                    {item.days_until === 0 ? 'Expires today' :
                                     item.days_until === 1 ? 'Expires tomorrow' :
                                     `Expires in ${item.days_until} days`}
                                </p>
                            </div>
                            {item.days_until <= 1 && (
                                <AlertTriangle className="w-4 h-4 text-red-500 dark:text-red-400" />
                            )}
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
