import { useState, useEffect, useMemo } from 'react';
import { Eye, EyeOff, Check, X, AlertCircle } from 'lucide-react';
import clsx from 'clsx';

export interface PasswordPolicy {
    min_length: number;
    require_uppercase: boolean;
    require_lowercase: boolean;
    require_number: boolean;
    require_special: boolean;
    max_age_days: number | null;
    prevent_reuse: number;
}

interface PasswordInputProps {
    value: string;
    onChange: (value: string) => void;
    policy?: PasswordPolicy | null;
    label?: string;
    placeholder?: string;
    showRequirements?: boolean;
    error?: string | string[];
    disabled?: boolean;
    autoComplete?: string;
    id?: string;
    name?: string;
}

const DEFAULT_POLICY: PasswordPolicy = {
    min_length: 8,
    require_uppercase: true,
    require_lowercase: true,
    require_number: true,
    require_special: false,
    max_age_days: null,
    prevent_reuse: 0,
};

export function PasswordInput({
    value,
    onChange,
    policy = DEFAULT_POLICY,
    label = 'Password',
    placeholder = '••••••••',
    showRequirements = true,
    error,
    disabled = false,
    autoComplete = 'new-password',
    id,
    name,
}: PasswordInputProps) {
    const [showPassword, setShowPassword] = useState(false);
    const [isFocused, setIsFocused] = useState(false);

    const effectivePolicy = policy || DEFAULT_POLICY;

    // Calculate which requirements are met
    const requirements = useMemo(() => {
        const reqs = [];
        
        reqs.push({
            label: `At least ${effectivePolicy.min_length} characters`,
            met: value.length >= effectivePolicy.min_length,
            required: true,
        });

        if (effectivePolicy.require_uppercase) {
            reqs.push({
                label: 'One uppercase letter',
                met: /[A-Z]/.test(value),
                required: true,
            });
        }

        if (effectivePolicy.require_lowercase) {
            reqs.push({
                label: 'One lowercase letter',
                met: /[a-z]/.test(value),
                required: true,
            });
        }

        if (effectivePolicy.require_number) {
            reqs.push({
                label: 'One number',
                met: /[0-9]/.test(value),
                required: true,
            });
        }

        if (effectivePolicy.require_special) {
            reqs.push({
                label: 'One special character (!@#$%^&*)',
                met: /[!@#$%^&*()_+\-=\[\]{};':"\\|,.<>\/?]/.test(value),
                required: true,
            });
        }

        return reqs;
    }, [value, effectivePolicy]);

    const allRequirementsMet = requirements.every(r => r.met);
    const hasValue = value.length > 0;

    // Calculate password strength (0-100)
    const strength = useMemo(() => {
        if (!hasValue) return 0;
        
        let score = 0;
        const metCount = requirements.filter(r => r.met).length;
        score = (metCount / requirements.length) * 60;
        
        // Bonus for length
        if (value.length >= effectivePolicy.min_length + 4) score += 20;
        if (value.length >= effectivePolicy.min_length + 8) score += 20;
        
        return Math.min(100, score);
    }, [value, requirements, effectivePolicy.min_length, hasValue]);

    const strengthLabel = useMemo(() => {
        if (!hasValue) return '';
        if (strength < 40) return 'Weak';
        if (strength < 70) return 'Fair';
        if (strength < 90) return 'Good';
        return 'Strong';
    }, [strength, hasValue]);

    const strengthColor = useMemo(() => {
        if (strength < 40) return 'bg-red-500';
        if (strength < 70) return 'bg-yellow-500';
        if (strength < 90) return 'bg-blue-500';
        return 'bg-green-500';
    }, [strength]);

    const errorMessages = Array.isArray(error) ? error : error ? [error] : [];

    return (
        <div className="space-y-2">
            {label && (
                <label htmlFor={id} className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                    {label}
                </label>
            )}
            
            <div className="relative">
                <input
                    type={showPassword ? 'text' : 'password'}
                    id={id}
                    name={name}
                    value={value}
                    onChange={(e) => onChange(e.target.value)}
                    onFocus={() => setIsFocused(true)}
                    onBlur={() => setIsFocused(false)}
                    placeholder={placeholder}
                    disabled={disabled}
                    autoComplete={autoComplete}
                    className={clsx(
                        "w-full px-4 py-2.5 pr-12 border rounded-lg transition-colors",
                        "bg-white dark:bg-gray-700 text-gray-900 dark:text-white",
                        "focus:ring-2 focus:ring-primary-500 focus:border-primary-500",
                        errorMessages.length > 0
                            ? "border-red-500 dark:border-red-500"
                            : "border-gray-300 dark:border-gray-600",
                        disabled && "opacity-50 cursor-not-allowed"
                    )}
                />
                <button
                    type="button"
                    onClick={() => setShowPassword(!showPassword)}
                    className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                    tabIndex={-1}
                >
                    {showPassword ? (
                        <EyeOff className="w-5 h-5" />
                    ) : (
                        <Eye className="w-5 h-5" />
                    )}
                </button>
            </div>

            {/* Error messages from backend */}
            {errorMessages.length > 0 && (
                <div className="flex items-start gap-2 text-red-600 dark:text-red-400 text-sm">
                    <AlertCircle className="w-4 h-4 flex-shrink-0 mt-0.5" />
                    <div>
                        {errorMessages.map((msg, i) => (
                            <p key={i}>{msg}</p>
                        ))}
                    </div>
                </div>
            )}

            {/* Strength indicator */}
            {showRequirements && hasValue && (
                <div className="space-y-1">
                    <div className="flex items-center justify-between text-xs">
                        <span className="text-gray-500 dark:text-gray-400">Password strength</span>
                        <span className={clsx(
                            "font-medium",
                            strength < 40 && "text-red-600 dark:text-red-400",
                            strength >= 40 && strength < 70 && "text-yellow-600 dark:text-yellow-400",
                            strength >= 70 && strength < 90 && "text-blue-600 dark:text-blue-400",
                            strength >= 90 && "text-green-600 dark:text-green-400"
                        )}>
                            {strengthLabel}
                        </span>
                    </div>
                    <div className="h-1.5 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                        <div 
                            className={clsx("h-full transition-all duration-300", strengthColor)}
                            style={{ width: `${strength}%` }}
                        />
                    </div>
                </div>
            )}

            {/* Requirements checklist */}
            {showRequirements && (isFocused || hasValue) && (
                <div className="p-3 bg-gray-50 dark:bg-gray-800/50 rounded-lg border border-gray-200 dark:border-gray-700">
                    <p className="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2">
                        Password requirements:
                    </p>
                    <ul className="space-y-1">
                        {requirements.map((req, i) => (
                            <li 
                                key={i}
                                className={clsx(
                                    "flex items-center gap-2 text-sm transition-colors",
                                    req.met 
                                        ? "text-green-600 dark:text-green-400" 
                                        : "text-gray-500 dark:text-gray-400"
                                )}
                            >
                                {req.met ? (
                                    <Check className="w-4 h-4 flex-shrink-0" />
                                ) : (
                                    <X className="w-4 h-4 flex-shrink-0 text-gray-300 dark:text-gray-600" />
                                )}
                                <span>{req.label}</span>
                            </li>
                        ))}
                    </ul>
                </div>
            )}
        </div>
    );
}

// Hook to fetch password policy
export function usePasswordPolicy(domain?: string) {
    const [policy, setPolicy] = useState<PasswordPolicy | null>(null);
    const [loading, setLoading] = useState(true);

    useEffect(() => {
        const fetchPolicy = async () => {
            try {
                const params = new URLSearchParams();
                if (domain) params.set('domain', domain);
                
                // Try to get auth token
                const token = localStorage.getItem('token') || sessionStorage.getItem('token');
                const headers: HeadersInit = {
                    'Content-Type': 'application/json',
                };
                if (token) {
                    headers['Authorization'] = `Bearer ${token}`;
                }
                
                const response = await fetch(`/api/auth/password-policy?${params.toString()}`, {
                    headers,
                });
                
                if (response.ok) {
                    const data = await response.json();
                    setPolicy(data);
                }
            } catch (error) {
                console.error('Failed to fetch password policy:', error);
            } finally {
                setLoading(false);
            }
        };

        fetchPolicy();
    }, [domain]);

    return { policy, loading };
}

// Validate password against policy (client-side)
export function validatePassword(password: string, policy: PasswordPolicy): string[] {
    const errors: string[] = [];
    
    if (password.length < policy.min_length) {
        errors.push(`Password must be at least ${policy.min_length} characters`);
    }
    
    if (policy.require_uppercase && !/[A-Z]/.test(password)) {
        errors.push('Password must contain at least one uppercase letter');
    }
    
    if (policy.require_lowercase && !/[a-z]/.test(password)) {
        errors.push('Password must contain at least one lowercase letter');
    }
    
    if (policy.require_number && !/[0-9]/.test(password)) {
        errors.push('Password must contain at least one number');
    }
    
    if (policy.require_special && !/[!@#$%^&*()_+\-=\[\]{};':"\\|,.<>\/?]/.test(password)) {
        errors.push('Password must contain at least one special character');
    }
    
    return errors;
}

