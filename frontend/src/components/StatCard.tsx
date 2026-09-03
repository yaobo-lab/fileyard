import { LucideIcon, ArrowUpRight, ArrowDownRight } from 'lucide-react';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
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
        <Card className={clsx("shadow-xs transition-colors", className)}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
                <CardTitle className="text-sm font-medium text-muted-foreground">{title}</CardTitle>
                <Icon className="h-4 w-4 text-muted-foreground" />
            </CardHeader>
            <CardContent>
                <div className="text-2xl font-bold tracking-tight text-foreground">{value}</div>
                {trend && (
                    <div className="flex items-center text-xs text-muted-foreground mt-1">
                        <span
                            className={clsx(
                                "flex items-center font-medium mr-1",
                                trend.positive ? "text-emerald-600 dark:text-emerald-400" : "text-destructive"
                            )}
                        >
                            {trend.positive ? (
                                <ArrowUpRight className="h-3.5 w-3.5 mr-0.5" />
                            ) : (
                                <ArrowDownRight className="h-3.5 w-3.5 mr-0.5" />
                            )}
                            {trend.positive ? '+' : '-'}{trend.value}%
                        </span>
                        <span>{trend.label}</span>
                    </div>
                )}
            </CardContent>
        </Card>
    );
}
