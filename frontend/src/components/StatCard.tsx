import { LucideIcon, ArrowUpRight, ArrowDownRight } from 'lucide-react';
import clsx from 'clsx';

interface StatCardProps {
    title: string;
    value: string | number;
    icon: LucideIcon;
    trend?: {
        value: number;
        label: string;
        positive: boolean;
    };
    className?: string;
}

export function StatCard({ title, value, icon: Icon, trend, className }: StatCardProps) {
    return (
        <div className={clsx("bg-white dark:bg-gray-800 p-6 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm transition-colors duration-200", className)}>
            <div className="flex items-center justify-between">
                <div>
                    <p className="text-sm font-medium text-gray-500 dark:text-gray-400">{title}</p>
                </div>
                <div className="p-3 bg-primary-50 dark:bg-primary-900/20 rounded-full">
                    <Icon className="w-6 h-6 text-primary-600 dark:text-primary-400" />
                </div>
            </div>
            <div className="flex items-baseline">
                <div className="text-2xl font-semibold text-gray-900 dark:text-white">{value}</div>
                {trend && (
                    <div className={clsx(
                        "ml-2 flex items-baseline text-sm font-semibold",
                        trend.positive ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400"
                    )}>
                        {trend.positive ? <ArrowUpRight className="self-center flex-shrink-0 h-4 w-4 text-green-500 dark:text-green-400" /> : <ArrowDownRight className="self-center flex-shrink-0 h-4 w-4 text-red-500 dark:text-red-400" />}
                        <span className="sr-only">{trend.positive ? 'Increased' : 'Decreased'} by</span>
                        {trend.value}%
                        <span className="ml-1 text-gray-500 dark:text-gray-400 font-normal">{trend.label}</span>
                    </div>
                )}
            </div>
        </div>
    );
}
