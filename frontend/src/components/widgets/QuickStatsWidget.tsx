import { useState, useEffect } from 'react';
import { BarChart3, Building2, Users, FileText, HardDrive } from 'lucide-react';
import { useAuthFetch } from '../../context/AuthContext';

interface QuickStats {
    companies: number;
    users: number;
    files: number;
    storage: string;
}

export function QuickStatsWidget() {
    const [stats, setStats] = useState<QuickStats>({
        companies: 0,
        users: 0,
        files: 0,
        storage: '0 B'
    });
    const [isLoading, setIsLoading] = useState(true);
    const authFetch = useAuthFetch();

    useEffect(() => {
        fetchStats();
    }, []);

    const fetchStats = async () => {
        try {
            const res = await authFetch('/api/dashboard/stats');
            if (res.ok) {
                const data = await res.json();
                setStats({
                    companies: data.stats?.companies || 0,
                    users: data.stats?.users || 0,
                    files: data.stats?.files || 0,
                    storage: data.stats?.storage_used_formatted || '0 B'
                });
            }
        } catch (error) {
            console.error('Failed to fetch stats', error);
        } finally {
            setIsLoading(false);
        }
    };

    const statItems = [
        { label: 'Companies', value: stats.companies, icon: Building2, color: 'text-blue-600 dark:text-blue-400 bg-blue-100 dark:bg-blue-900/30' },
        { label: 'Users', value: stats.users, icon: Users, color: 'text-green-600 dark:text-green-400 bg-green-100 dark:bg-green-900/30' },
        { label: 'Files', value: stats.files, icon: FileText, color: 'text-purple-600 dark:text-purple-400 bg-purple-100 dark:bg-purple-900/30' },
        { label: 'Storage', value: stats.storage, icon: HardDrive, color: 'text-orange-600 dark:text-orange-400 bg-orange-100 dark:bg-orange-900/30' },
    ];

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-6 h-full">
            <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-medium text-gray-900 dark:text-white">Quick Stats</h3>
                <BarChart3 className="w-5 h-5 text-gray-400" />
            </div>
            
            {isLoading ? (
                <div className="grid grid-cols-2 gap-3">
                    {[...Array(4)].map((_, i) => (
                        <div key={i} className="h-16 bg-gray-200 dark:bg-gray-700 rounded-lg animate-pulse"></div>
                    ))}
                </div>
            ) : (
                <div className="grid grid-cols-2 gap-3">
                    {statItems.map((item) => {
                        const Icon = item.icon;
                        return (
                            <div key={item.label} className={`p-3 rounded-lg ${item.color}`}>
                                <div className="flex items-center mb-1">
                                    <Icon className="w-4 h-4 mr-1" />
                                    <span className="text-xs font-medium opacity-75">{item.label}</span>
                                </div>
                                <p className="text-xl font-bold">{item.value}</p>
                            </div>
                        );
                    })}
                </div>
            )}
        </div>
    );
}
