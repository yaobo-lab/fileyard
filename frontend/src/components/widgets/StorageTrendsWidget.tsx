import { useState, useEffect } from 'react';
import { TrendingUp, HardDrive } from 'lucide-react';
import { useAuthFetch } from '../../context/AuthContext';

interface StorageTrendsWidgetProps {
    period?: '7d' | '30d' | '90d';
}

export function StorageTrendsWidget({ period = '30d' }: StorageTrendsWidgetProps) {
    const [currentStorage, setCurrentStorage] = useState('0 B');
    const [isLoading, setIsLoading] = useState(true);
    const authFetch = useAuthFetch();

    useEffect(() => {
        fetchStorageData();
    }, [period]);

    const fetchStorageData = async () => {
        try {
            const res = await authFetch('/api/dashboard/stats');
            if (res.ok) {
                const data = await res.json();
                setCurrentStorage(data.stats?.storage_used_formatted || '0 B');
            }
        } catch (error) {
            console.error('Failed to fetch storage trends', error);
        } finally {
            setIsLoading(false);
        }
    };

    // Mock trend data for visualization
    const trendData = [
        { label: 'Week 1', value: 25 },
        { label: 'Week 2', value: 35 },
        { label: 'Week 3', value: 45 },
        { label: 'Week 4', value: 60 },
    ];

    const maxValue = Math.max(...trendData.map(d => d.value));

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-6 h-full">
            <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-medium text-gray-900 dark:text-white">Storage Trends</h3>
                <TrendingUp className="w-5 h-5 text-gray-400" />
            </div>
            
            {isLoading ? (
                <div className="space-y-4">
                    <div className="h-8 bg-gray-200 dark:bg-gray-700 rounded animate-pulse"></div>
                    <div className="h-32 bg-gray-200 dark:bg-gray-700 rounded animate-pulse"></div>
                </div>
            ) : (
                <div className="space-y-4">
                    {/* Current Storage */}
                    <div className="flex items-center p-3 bg-primary-50 dark:bg-primary-900/20 rounded-lg">
                        <HardDrive className="w-6 h-6 text-primary-600 dark:text-primary-400 mr-3" />
                        <div>
                            <p className="text-xs text-primary-600 dark:text-primary-400 font-medium">Current Usage</p>
                            <p className="text-lg font-bold text-primary-700 dark:text-primary-300">{currentStorage}</p>
                        </div>
                    </div>

                    {/* Simple Bar Chart */}
                    <div>
                        <p className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-3">
                            Last {period === '7d' ? '7 Days' : period === '30d' ? '30 Days' : '90 Days'}
                        </p>
                        <div className="flex items-end justify-between h-24 gap-2">
                            {trendData.map((item, index) => (
                                <div key={index} className="flex-1 flex flex-col items-center">
                                    <div 
                                        className="w-full bg-primary-500 dark:bg-primary-600 rounded-t transition-all duration-500"
                                        style={{ height: `${(item.value / maxValue) * 100}%` }}
                                    ></div>
                                    <span className="text-xs text-gray-400 mt-1">W{index + 1}</span>
                                </div>
                            ))}
                        </div>
                    </div>

                    <p className="text-xs text-gray-400 dark:text-gray-500 text-center">
                        Historical data tracking coming soon
                    </p>
                </div>
            )}
        </div>
    );
}
