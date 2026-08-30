import { useState, useEffect } from 'react';
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, Cell } from 'recharts';
import { Activity } from 'lucide-react';
import { useAuthFetch } from '../../context/AuthContext';
import { format, subDays, parseISO, startOfDay } from 'date-fns';

interface ActivityLog {
    id: string;
    timestamp: string;
    action: string;
}

interface DayActivity {
    day: string;
    shortDay: string;
    count: number;
}

interface ActivityChartWidgetProps {
    days?: number;
}

export function ActivityChartWidget({ days = 7 }: ActivityChartWidgetProps) {
    const [activityData, setActivityData] = useState<DayActivity[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [totalActivity, setTotalActivity] = useState(0);
    const authFetch = useAuthFetch();

    useEffect(() => {
        fetchActivityData();
    }, [days]);

    const fetchActivityData = async () => {
        try {
            // Fetch more logs to get a good sample for the last N days
            const res = await authFetch(`/api/activity-logs?limit=500`);
            if (res.ok) {
                const data = await res.json();
                const logs: ActivityLog[] = data.logs || [];
                
                // Group by day
                const today = startOfDay(new Date());
                const dayMap = new Map<string, number>();
                
                // Initialize all days with 0
                for (let i = days - 1; i >= 0; i--) {
                    const date = subDays(today, i);
                    const dayKey = format(date, 'yyyy-MM-dd');
                    dayMap.set(dayKey, 0);
                }
                
                // Count activities per day
                logs.forEach(log => {
                    const logDate = format(startOfDay(parseISO(log.timestamp)), 'yyyy-MM-dd');
                    if (dayMap.has(logDate)) {
                        dayMap.set(logDate, (dayMap.get(logDate) || 0) + 1);
                    }
                });
                
                // Convert to array
                const chartData: DayActivity[] = [];
                let total = 0;
                dayMap.forEach((count, dayKey) => {
                    const date = parseISO(dayKey);
                    chartData.push({
                        day: dayKey,
                        shortDay: format(date, 'EEE'),
                        count
                    });
                    total += count;
                });
                
                setActivityData(chartData);
                setTotalActivity(total);
            }
        } catch (error) {
            console.error('Failed to fetch activity data', error);
        } finally {
            setIsLoading(false);
        }
    };

    const maxCount = Math.max(...activityData.map(d => d.count), 1);

    const CustomTooltip = ({ active, payload, label }: any) => {
        if (active && payload && payload.length) {
            return (
                <div className="bg-gray-900 dark:bg-gray-700 text-white px-3 py-2 rounded-lg shadow-lg text-sm">
                    <p className="font-medium">{format(parseISO(payload[0].payload.day), 'MMM d, yyyy')}</p>
                    <p className="text-primary-300">{payload[0].value} activities</p>
                </div>
            );
        }
        return null;
    };

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-6 h-full">
            <div className="flex items-center justify-between mb-4">
                <div>
                    <h3 className="text-lg font-medium text-gray-900 dark:text-white">Activity Overview</h3>
                    <p className="text-sm text-gray-500 dark:text-gray-400">Last {days} days</p>
                </div>
                <div className="flex items-center space-x-2">
                    <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-lg">
                        <Activity className="w-5 h-5 text-primary-600 dark:text-primary-400" />
                    </div>
                    <div className="text-right">
                        <p className="text-2xl font-bold text-gray-900 dark:text-white">{totalActivity}</p>
                        <p className="text-xs text-gray-500 dark:text-gray-400">Total</p>
                    </div>
                </div>
            </div>
            
            {isLoading ? (
                <div className="h-48 flex items-center justify-center">
                    <div className="animate-pulse w-full h-32 bg-gray-200 dark:bg-gray-700 rounded"></div>
                </div>
            ) : activityData.length === 0 ? (
                <div className="h-48 flex items-center justify-center text-gray-500 dark:text-gray-400">
                    <div className="text-center">
                        <Activity className="w-8 h-8 mx-auto mb-2 opacity-50" />
                        <p className="text-sm">No activity data available</p>
                    </div>
                </div>
            ) : (
                <div className="h-[200px] w-full">
                    <ResponsiveContainer width="100%" height={200}>
                        <BarChart data={activityData} margin={{ top: 10, right: 10, left: -20, bottom: 0 }}>
                            <XAxis 
                                dataKey="shortDay" 
                                axisLine={false}
                                tickLine={false}
                                tick={{ fill: '#9CA3AF', fontSize: 12 }}
                            />
                            <YAxis 
                                axisLine={false}
                                tickLine={false}
                                tick={{ fill: '#9CA3AF', fontSize: 12 }}
                                allowDecimals={false}
                            />
                            <Tooltip content={<CustomTooltip />} cursor={{ fill: 'rgba(59, 130, 246, 0.1)' }} />
                            <Bar 
                                dataKey="count" 
                                radius={[4, 4, 0, 0]}
                                maxBarSize={40}
                            >
                                {activityData.map((entry, index) => (
                                    <Cell 
                                        key={`cell-${index}`}
                                        fill={entry.count === maxCount ? '#3B82F6' : '#60A5FA'}
                                        className="transition-all duration-300 hover:opacity-80"
                                    />
                                ))}
                            </Bar>
                        </BarChart>
                    </ResponsiveContainer>
                </div>
            )}
        </div>
    );
}
