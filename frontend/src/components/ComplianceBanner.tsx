import { Shield, AlertTriangle, Info } from 'lucide-react';
import clsx from 'clsx';

interface ComplianceBannerProps {
    mode: string;
    className?: string;
}

export function ComplianceBanner({ mode, className }: ComplianceBannerProps) {
    // Don't show banner for standard mode
    if (!mode || mode === 'Standard' || mode === 'None' || mode === 'none') {
        return null;
    }

    const getModeInfo = () => {
        switch (mode.toUpperCase()) {
            case 'HIPAA':
                return {
                    label: 'HIPAA Secure',
                    description: 'Healthcare data protection mode is active. MFA required, public sharing disabled, all access logged.',
                    color: 'green',
                    icon: Shield,
                };
            case 'SOX':
            case 'SOC2':
                return {
                    label: 'SOX Governed',
                    description: 'Financial audit compliance mode is active. File versioning enabled, audit logging enforced.',
                    color: 'blue',
                    icon: Shield,
                };
            case 'GDPR':
                return {
                    label: 'GDPR Active',
                    description: 'European data protection mode is active. Consent tracking and data deletion requests enabled.',
                    color: 'purple',
                    icon: Shield,
                };
            default:
                return {
                    label: 'Compliance Mode',
                    description: 'Compliance mode is active.',
                    color: 'yellow',
                    icon: AlertTriangle,
                };
        }
    };

    const info = getModeInfo();
    const Icon = info.icon;

    const colorClasses = {
        green: 'bg-green-50 dark:bg-green-900/20 border-green-200 dark:border-green-800 text-green-800 dark:text-green-200',
        blue: 'bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800 text-blue-800 dark:text-blue-200',
        purple: 'bg-purple-50 dark:bg-purple-900/20 border-purple-200 dark:border-purple-800 text-purple-800 dark:text-purple-200',
        yellow: 'bg-yellow-50 dark:bg-yellow-900/20 border-yellow-200 dark:border-yellow-800 text-yellow-800 dark:text-yellow-200',
    };

    const iconColorClasses = {
        green: 'text-green-600 dark:text-green-400',
        blue: 'text-blue-600 dark:text-blue-400',
        purple: 'text-purple-600 dark:text-purple-400',
        yellow: 'text-yellow-600 dark:text-yellow-400',
    };

    return (
        <div className={clsx(
            'border rounded-lg p-4 flex items-start gap-3',
            colorClasses[info.color as keyof typeof colorClasses],
            className
        )}>
            <Icon className={clsx('w-5 h-5 flex-shrink-0 mt-0.5', iconColorClasses[info.color as keyof typeof iconColorClasses])} />
            <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                    <span className="font-semibold">{info.label}</span>
                    <span className="text-xs opacity-75">— Compliance Enforcement Active</span>
                </div>
                <p className="text-sm mt-1 opacity-90">
                    {info.description}
                </p>
                <p className="text-xs mt-2 opacity-75 flex items-center gap-1">
                    <Info className="w-3 h-3" />
                    Some settings are restricted due to compliance requirements.
                </p>
            </div>
        </div>
    );
}

export default ComplianceBanner;
