import { Lock, Info } from 'lucide-react';
import clsx from 'clsx';
import { useState } from 'react';

interface LockedFieldProps {
    label: string;
    locked: boolean;
    reason?: string;
    children: React.ReactNode;
    className?: string;
}

export function LockedField({ label, locked, reason, children, className }: LockedFieldProps) {
    const [showTooltip, setShowTooltip] = useState(false);

    const defaultReason = 'Restricted due to Compliance Mode';

    return (
        <div className={clsx('relative', className)}>
            <div className="flex items-center justify-between mb-1">
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    {label}
                </label>
                {locked && (
                    <div 
                        className="relative"
                        onMouseEnter={() => setShowTooltip(true)}
                        onMouseLeave={() => setShowTooltip(false)}
                    >
                        <div className="flex items-center gap-1 text-amber-600 dark:text-amber-400 cursor-help">
                            <Lock className="w-3.5 h-3.5" />
                            <span className="text-xs">Locked</span>
                        </div>
                        {showTooltip && (
                            <div className="absolute z-50 right-0 top-full mt-1 w-64 p-2 text-xs text-white bg-gray-900 dark:bg-gray-700 rounded-lg shadow-lg">
                                <div className="flex items-start gap-2">
                                    <Info className="w-3.5 h-3.5 flex-shrink-0 mt-0.5" />
                                    <span>{reason || defaultReason}</span>
                                </div>
                                <div className="absolute -top-1 right-4 w-2 h-2 bg-gray-900 dark:bg-gray-700 rotate-45"></div>
                            </div>
                        )}
                    </div>
                )}
            </div>
            <div className={clsx(
                'relative',
                locked && 'opacity-60 pointer-events-none'
            )}>
                {children}
                {locked && (
                    <div className="absolute inset-0 cursor-not-allowed" />
                )}
            </div>
        </div>
    );
}

interface LockedToggleProps {
    label: string;
    description?: string;
    checked: boolean;
    onChange: (checked: boolean) => void;
    locked: boolean;
    reason?: string;
    className?: string;
}

export function LockedToggle({ 
    label, 
    description, 
    checked, 
    onChange, 
    locked, 
    reason,
    className 
}: LockedToggleProps) {
    const [showTooltip, setShowTooltip] = useState(false);
    const defaultReason = 'Restricted due to Compliance Mode';

    return (
        <label 
            className={clsx(
                'flex items-center justify-between p-4 border rounded-lg transition-colors',
                locked 
                    ? 'border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50 cursor-not-allowed' 
                    : 'border-gray-200 dark:border-gray-600 cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700/50',
                className
            )}
        >
            <div className="flex-1">
                <span className={clsx(
                    'block text-sm font-medium',
                    locked 
                        ? 'text-gray-500 dark:text-gray-500' 
                        : 'text-gray-900 dark:text-white'
                )}>
                    {label}
                </span>
                {description && (
                    <span className="block text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                        {description}
                    </span>
                )}
            </div>
            <div className="flex items-center gap-2">
                {locked && (
                    <div 
                        className="relative"
                        onMouseEnter={() => setShowTooltip(true)}
                        onMouseLeave={() => setShowTooltip(false)}
                    >
                        <Lock className="w-4 h-4 text-amber-600 dark:text-amber-400 cursor-help" />
                        {showTooltip && (
                            <div className="absolute z-50 right-0 bottom-full mb-1 w-64 p-2 text-xs text-white bg-gray-900 dark:bg-gray-700 rounded-lg shadow-lg">
                                <div className="flex items-start gap-2">
                                    <Info className="w-3.5 h-3.5 flex-shrink-0 mt-0.5" />
                                    <span>{reason || defaultReason}</span>
                                </div>
                                <div className="absolute -bottom-1 right-2 w-2 h-2 bg-gray-900 dark:bg-gray-700 rotate-45"></div>
                            </div>
                        )}
                    </div>
                )}
                <input
                    type="checkbox"
                    checked={checked}
                    onChange={(e) => !locked && onChange(e.target.checked)}
                    disabled={locked}
                    className={clsx(
                        'form-checkbox h-5 w-5 rounded border-gray-300 dark:border-gray-600 dark:bg-gray-700',
                        locked 
                            ? 'text-gray-400 cursor-not-allowed' 
                            : 'text-primary-600 cursor-pointer'
                    )}
                />
            </div>
        </label>
    );
}

export default LockedField;
