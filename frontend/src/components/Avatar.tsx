import { useState, useMemo, useEffect } from 'react';
import clsx from 'clsx';

interface AvatarProps {
    src?: string | null;
    name: string;
    size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl';
    className?: string;
}

function getInitials(name: string): string {
    if (!name) return '?';
    const parts = name.trim().split(/\s+/);
    if (parts.length === 1) {
        return parts[0].substring(0, 2).toUpperCase();
    }
    return (parts[0][0] + parts[parts.length - 1][0]).toUpperCase();
}

const sizeClasses = {
    xs: 'h-6 w-6 text-xs',
    sm: 'h-8 w-8 text-xs',
    md: 'h-10 w-10 text-sm',
    lg: 'h-12 w-12 text-base',
    xl: 'h-16 w-16 text-lg',
};

export function Avatar({ src, name, size = 'md', className }: AvatarProps) {
    const [imgError, setImgError] = useState(false);
    const cacheBuster = useMemo(() => Date.now(), [src]);
    
    // Reset error state when src changes
    useEffect(() => {
        setImgError(false);
    }, [src]);
    
    const baseClasses = clsx(
        'rounded-full flex items-center justify-center font-medium',
        sizeClasses[size],
        className
    );

    // Show fallback if no src or if image failed to load
    if (!src || imgError) {
        return (
            <div className={clsx(baseClasses, 'bg-primary-600 text-white')}>
                {getInitials(name)}
            </div>
        );
    }

    return (
        <img
            src={`${src}?t=${cacheBuster}`}
            alt=""
            className={clsx(baseClasses, 'object-cover')}
            onError={() => setImgError(true)}
        />
    );
}
