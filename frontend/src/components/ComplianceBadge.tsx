import { Shield, Lock, ShieldCheck } from 'lucide-react';
import clsx from 'clsx';

interface ComplianceBadgeProps {
    mode: string;
    size?: 'sm' | 'md' | 'lg';
    showLabel?: boolean;
    className?: string;
}

export function ComplianceBadge({ mode, size = 'md', showLabel = true, className }: ComplianceBadgeProps) {
    // Don't show badge for standard mode
    if (!mode || mode === 'Standard' || mode === 'None' || mode === 'none') {
        return null;
    }

    const getBadgeInfo = () => {
        switch (mode.toUpperCase()) {
            case 'HIPAA':
                return {
                    label: 'HIPAA Secure',
                    shortLabel: 'HIPAA',
                    icon: ShieldCheck,
                    bgColor: 'bg-green-100 dark:bg-green-900/30',
                    textColor: 'text-green-700 dark:text-green-400',
                    borderColor: 'border-green-200 dark:border-green-800',
                };
            case 'SOX':
            case 'SOC2':
                return {
                    label: 'SOX Governed',
                    shortLabel: 'SOX',
                    icon: Lock,
                    bgColor: 'bg-blue-100 dark:bg-blue-900/30',
                    textColor: 'text-blue-700 dark:text-blue-400',
                    borderColor: 'border-blue-200 dark:border-blue-800',
                };
            case 'GDPR':
                return {
                    label: 'GDPR Active',
                    shortLabel: 'GDPR',
                    icon: Shield,
                    bgColor: 'bg-purple-100 dark:bg-purple-900/30',
                    textColor: 'text-purple-700 dark:text-purple-400',
                    borderColor: 'border-purple-200 dark:border-purple-800',
                };
            default:
                return {
                    label: 'Compliance',
                    shortLabel: mode,
                    icon: Shield,
                    bgColor: 'bg-gray-100 dark:bg-gray-800',
                    textColor: 'text-gray-700 dark:text-gray-400',
                    borderColor: 'border-gray-200 dark:border-gray-700',
                };
        }
    };

    const info = getBadgeInfo();
    const Icon = info.icon;

    const sizeClasses = {
        sm: 'text-xs px-1.5 py-0.5 gap-1',
        md: 'text-xs px-2 py-1 gap-1.5',
        lg: 'text-sm px-3 py-1.5 gap-2',
    };

    const iconSizes = {
        sm: 'w-3 h-3',
        md: 'w-3.5 h-3.5',
        lg: 'w-4 h-4',
    };

    return (
        <span
            className={clsx(
                'inline-flex items-center font-medium rounded-full border',
                info.bgColor,
                info.textColor,
                info.borderColor,
                sizeClasses[size],
                className
            )}
        >
            <Icon className={iconSizes[size]} />
            {showLabel && <span>{size === 'sm' ? info.shortLabel : info.label}</span>}
        </span>
    );
}

export default ComplianceBadge;
