import { useState } from 'react';
import { 
    X, 
    User, 
    Mail, 
    Shield, 
    Ban, 
    UserCheck, 
    Trash2, 
    Calendar, 
    Clock,
    AlertTriangle,
    ChevronDown,
    ChevronUp,
    Key,
    Send,
    Eye,
    EyeOff
} from 'lucide-react';
import clsx from 'clsx';
import { useGlobalSettings } from '../context/GlobalSettingsContext';
import { usePasswordPolicy, validatePassword } from './PasswordInput';

interface ManageUserModalProps {
    isOpen: boolean;
    onClose: () => void;
    user: {
        id: string;
        name: string;
        email: string;
        role: string;
        status: string;
        suspended_at?: string | null;
        suspended_until?: string | null;
    } | null;
    onSuspend: (data: { until: string | null; reason: string }) => Promise<void>;
    onUnsuspend: () => Promise<void>;
    onPermanentDelete: () => Promise<void>;
    onResetPassword?: (newPassword: string) => Promise<void>;
    onSendResetEmail?: () => Promise<void>;
    onChangeEmail?: (newEmail: string) => Promise<void>;
    canSuspend: boolean;
    canDelete: boolean;
    canResetPassword?: boolean;
}

type ActionType = 'none' | 'suspend' | 'delete' | 'password' | 'email';

export function ManageUserModal({ 
    isOpen, 
    onClose, 
    user,
    onSuspend,
    onUnsuspend,
    onPermanentDelete,
    onResetPassword,
    onSendResetEmail,
    onChangeEmail,
    canSuspend,
    canDelete,
    canResetPassword = false
}: ManageUserModalProps) {
    const { formatDateTime } = useGlobalSettings();
    const [activeAction, setActiveAction] = useState<ActionType>('none');
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [successMessage, setSuccessMessage] = useState<string | null>(null);
    
    // Suspend form state
    const [suspensionType, setSuspensionType] = useState<'indefinite' | 'timed'>('indefinite');
    const [untilDate, setUntilDate] = useState('');
    const [untilTime, setUntilTime] = useState('23:59');
    const [suspendReason, setSuspendReason] = useState('');
    
    // Delete form state
    const [confirmEmail, setConfirmEmail] = useState('');
    
    // Password reset form state
    const [newPassword, setNewPassword] = useState('');
    const [confirmPassword, setConfirmPassword] = useState('');
    const [showPassword, setShowPassword] = useState(false);
    const [passwordErrors, setPasswordErrors] = useState<string[]>([]);
    
    // Fetch password policy
    const { policy: passwordPolicy } = usePasswordPolicy();
    
    // Change email form state
    const [newEmail, setNewEmail] = useState('');

    const isUserSuspended = user?.suspended_at && (
        !user.suspended_until || new Date(user.suspended_until) > new Date()
    );

    const getSuspensionInfo = () => {
        if (!user?.suspended_until) return 'Indefinitely';
        return `Until ${formatDateTime(user.suspended_until)}`;
    };

    const today = new Date().toISOString().split('T')[0];

    const resetForm = () => {
        setActiveAction('none');
        setError(null);
        setSuccessMessage(null);
        setSuspensionType('indefinite');
        setUntilDate('');
        setUntilTime('23:59');
        setSuspendReason('');
        setConfirmEmail('');
        setNewPassword('');
        setConfirmPassword('');
        setShowPassword(false);
        setNewEmail('');
        setPasswordErrors([]);
    };

    const handleClose = () => {
        resetForm();
        onClose();
    };

    const handleSuspend = async () => {
        setError(null);
        setIsSubmitting(true);

        try {
            let until: string | null = null;
            
            if (suspensionType === 'timed') {
                if (!untilDate) {
                    setError('Please select an end date for the suspension');
                    setIsSubmitting(false);
                    return;
                }
                until = new Date(`${untilDate}T${untilTime}`).toISOString();
            }

            await onSuspend({ until, reason: suspendReason.trim() });
            handleClose();
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to suspend user');
        } finally {
            setIsSubmitting(false);
        }
    };

    const handleUnsuspend = async () => {
        setError(null);
        setIsSubmitting(true);

        try {
            await onUnsuspend();
            handleClose();
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to unsuspend user');
        } finally {
            setIsSubmitting(false);
        }
    };

    const handlePermanentDelete = async () => {
        if (confirmEmail !== user?.email) {
            setError('Email does not match. Please type the user\'s email exactly to confirm.');
            return;
        }

        setError(null);
        setIsSubmitting(true);

        try {
            await onPermanentDelete();
            handleClose();
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to delete user');
        } finally {
            setIsSubmitting(false);
        }
    };

    const handleResetPassword = async () => {
        setPasswordErrors([]);
        
        // Validate against password policy
        if (passwordPolicy) {
            const errors = validatePassword(newPassword, passwordPolicy);
            if (errors.length > 0) {
                setPasswordErrors(errors);
                setError('Password does not meet requirements');
                return;
            }
        } else if (newPassword.length < 8) {
            setError('Password must be at least 8 characters long');
            return;
        }
        
        if (newPassword !== confirmPassword) {
            setError('Passwords do not match');
            return;
        }

        setError(null);
        setIsSubmitting(true);

        try {
            await onResetPassword?.(newPassword);
            setSuccessMessage('Password has been reset successfully');
            setNewPassword('');
            setConfirmPassword('');
            setPasswordErrors([]);
            setActiveAction('none');
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to reset password');
        } finally {
            setIsSubmitting(false);
        }
    };

    const handleSendResetEmail = async () => {
        setError(null);
        setIsSubmitting(true);

        try {
            await onSendResetEmail?.();
            setSuccessMessage('Password reset email has been sent');
            setActiveAction('none');
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to send reset email');
        } finally {
            setIsSubmitting(false);
        }
    };

    const handleChangeEmail = async () => {
        if (!newEmail || !newEmail.includes('@')) {
            setError('Please enter a valid email address');
            return;
        }

        setError(null);
        setIsSubmitting(true);

        try {
            await onChangeEmail?.(newEmail);
            setSuccessMessage('Email has been updated successfully');
            setNewEmail('');
            setActiveAction('none');
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to change email');
        } finally {
            setIsSubmitting(false);
        }
    };

    const toggleAction = (action: ActionType) => {
        if (activeAction === action) {
            setActiveAction('none');
            setError(null);
            setSuccessMessage(null);
        } else {
            setActiveAction(action);
            setError(null);
            setSuccessMessage(null);
        }
    };

    if (!isOpen || !user) return null;

    return (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-lg w-full mx-4 max-h-[90vh] overflow-hidden flex flex-col">
                {/* Header */}
                <div className="p-6 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between flex-shrink-0">
                    <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
                        Manage User
                    </h2>
                    <button
                        onClick={handleClose}
                        className="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-full transition-colors"
                    >
                        <X className="w-5 h-5 text-gray-500" />
                    </button>
                </div>

                {/* Content - Scrollable */}
                <div className="p-6 space-y-4 overflow-y-auto flex-1">
                    {/* User Info Card */}
                    <div className="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-4">
                        <div className="flex items-center gap-4">
                            <div className="h-12 w-12 rounded-full bg-primary-600 flex items-center justify-center text-white font-semibold">
                                {user.name.split(' ').map(n => n[0]).join('').toUpperCase().slice(0, 2)}
                            </div>
                            <div className="flex-1">
                                <h3 className="font-medium text-gray-900 dark:text-white flex items-center gap-2">
                                    <User className="w-4 h-4 text-gray-400" />
                                    {user.name}
                                </h3>
                                <p className="text-sm text-gray-500 dark:text-gray-400 flex items-center gap-2">
                                    <Mail className="w-3 h-3" />
                                    {user.email}
                                </p>
                            </div>
                            <div className="text-right">
                                <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-300">
                                    <Shield className="w-3 h-3 mr-1" />
                                    {user.role}
                                </span>
                                {isUserSuspended && (
                                    <p className="text-xs text-amber-600 dark:text-amber-400 mt-1 flex items-center justify-end gap-1">
                                        <Ban className="w-3 h-3" />
                                        Suspended {getSuspensionInfo()}
                                    </p>
                                )}
                            </div>
                        </div>
                    </div>

                    {error && (
                        <div className="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-600 dark:text-red-400">
                            {error}
                        </div>
                    )}

                    {successMessage && (
                        <div className="p-3 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg text-sm text-green-600 dark:text-green-400">
                            {successMessage}
                        </div>
                    )}

                    {/* Actions */}
                    <div className="space-y-3">
                        {/* Reset Password Action */}
                        {canResetPassword && (
                            <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
                                <button
                                    onClick={() => toggleAction('password')}
                                    className="w-full p-4 flex items-center justify-between text-left hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
                                >
                                    <div className="flex items-center gap-3">
                                        <Key className="w-5 h-5 text-blue-600 dark:text-blue-400" />
                                        <div>
                                            <p className="font-medium text-gray-900 dark:text-white">Reset Password</p>
                                            <p className="text-xs text-gray-500 dark:text-gray-400">
                                                Set a new password or send a reset link
                                            </p>
                                        </div>
                                    </div>
                                    {activeAction === 'password' 
                                        ? <ChevronUp className="w-5 h-5 text-gray-400" />
                                        : <ChevronDown className="w-5 h-5 text-gray-400" />
                                    }
                                </button>

                                {activeAction === 'password' && (
                                    <div className="p-4 bg-gray-50 dark:bg-gray-700/30 border-t border-gray-200 dark:border-gray-700 space-y-4">
                                        {/* Set Password Directly */}
                                        <div className="space-y-3">
                                            <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300">Set New Password</h4>
                                            <div className="relative">
                                                <input
                                                    type={showPassword ? 'text' : 'password'}
                                                    value={newPassword}
                                                    onChange={(e) => {
                                                        setNewPassword(e.target.value);
                                                        setPasswordErrors([]);
                                                    }}
                                                    placeholder={`New password (min ${passwordPolicy?.min_length || 8} characters)`}
                                                    className={clsx(
                                                        "w-full px-3 py-2 pr-10 text-sm border rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white",
                                                        passwordErrors.length > 0
                                                            ? "border-red-500"
                                                            : "border-gray-300 dark:border-gray-600"
                                                    )}
                                                />
                                                <button
                                                    type="button"
                                                    onClick={() => setShowPassword(!showPassword)}
                                                    className="absolute right-2 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600"
                                                >
                                                    {showPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                                                </button>
                                            </div>
                                            
                                            {/* Password errors */}
                                            {passwordErrors.length > 0 && (
                                                <div className="p-2 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg">
                                                    <ul className="text-xs text-red-600 dark:text-red-400 space-y-1">
                                                        {passwordErrors.map((err, i) => (
                                                            <li key={i}>• {err}</li>
                                                        ))}
                                                    </ul>
                                                </div>
                                            )}
                                            
                                            {/* Password requirements */}
                                            {passwordPolicy && newPassword && (
                                                <div className="p-2 bg-gray-100 dark:bg-gray-800/50 rounded-lg border border-gray-200 dark:border-gray-600">
                                                    <p className="text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">Requirements:</p>
                                                    <ul className="text-xs space-y-0.5">
                                                        <li className={newPassword.length >= passwordPolicy.min_length ? "text-green-600 dark:text-green-400" : "text-gray-400"}>
                                                            {newPassword.length >= passwordPolicy.min_length ? "✓" : "○"} {passwordPolicy.min_length}+ characters
                                                        </li>
                                                        {passwordPolicy.require_uppercase && (
                                                            <li className={/[A-Z]/.test(newPassword) ? "text-green-600 dark:text-green-400" : "text-gray-400"}>
                                                                {/[A-Z]/.test(newPassword) ? "✓" : "○"} Uppercase letter
                                                            </li>
                                                        )}
                                                        {passwordPolicy.require_lowercase && (
                                                            <li className={/[a-z]/.test(newPassword) ? "text-green-600 dark:text-green-400" : "text-gray-400"}>
                                                                {/[a-z]/.test(newPassword) ? "✓" : "○"} Lowercase letter
                                                            </li>
                                                        )}
                                                        {passwordPolicy.require_number && (
                                                            <li className={/[0-9]/.test(newPassword) ? "text-green-600 dark:text-green-400" : "text-gray-400"}>
                                                                {/[0-9]/.test(newPassword) ? "✓" : "○"} Number
                                                            </li>
                                                        )}
                                                        {passwordPolicy.require_special && (
                                                            <li className={/[!@#$%^&*()_+\-=\[\]{};':"\\|,.<>\/?]/.test(newPassword) ? "text-green-600 dark:text-green-400" : "text-gray-400"}>
                                                                {/[!@#$%^&*()_+\-=\[\]{};':"\\|,.<>\/?]/.test(newPassword) ? "✓" : "○"} Special character
                                                            </li>
                                                        )}
                                                    </ul>
                                                </div>
                                            )}
                                            
                                            <input
                                                type={showPassword ? 'text' : 'password'}
                                                value={confirmPassword}
                                                onChange={(e) => setConfirmPassword(e.target.value)}
                                                placeholder="Confirm new password"
                                                className={clsx(
                                                    "w-full px-3 py-2 text-sm border rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white",
                                                    confirmPassword && newPassword !== confirmPassword
                                                        ? "border-red-500"
                                                        : "border-gray-300 dark:border-gray-600"
                                                )}
                                            />
                                            {confirmPassword && newPassword !== confirmPassword && (
                                                <p className="text-xs text-red-600 dark:text-red-400">Passwords do not match</p>
                                            )}
                                            <button
                                                onClick={handleResetPassword}
                                                disabled={isSubmitting || !newPassword || newPassword !== confirmPassword}
                                                className="w-full px-4 py-2 bg-blue-600 text-white rounded-lg text-sm font-medium hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                                            >
                                                <Key className="w-4 h-4" />
                                                {isSubmitting ? 'Setting Password...' : 'Set Password'}
                                            </button>
                                        </div>

                                        <div className="relative">
                                            <div className="absolute inset-0 flex items-center">
                                                <div className="w-full border-t border-gray-300 dark:border-gray-600" />
                                            </div>
                                            <div className="relative flex justify-center text-xs">
                                                <span className="px-2 bg-gray-50 dark:bg-gray-700/30 text-gray-500">or</span>
                                            </div>
                                        </div>

                                        {/* Send Reset Email */}
                                        <div className="space-y-3">
                                            <h4 className="text-sm font-medium text-gray-700 dark:text-gray-300">Send Reset Link</h4>
                                            <p className="text-xs text-gray-500 dark:text-gray-400">
                                                Send a password reset email to {user?.email}
                                            </p>
                                            <button
                                                onClick={handleSendResetEmail}
                                                disabled={isSubmitting}
                                                className="w-full px-4 py-2 bg-gray-200 dark:bg-gray-600 text-gray-800 dark:text-gray-200 rounded-lg text-sm font-medium hover:bg-gray-300 dark:hover:bg-gray-500 disabled:opacity-50 flex items-center justify-center gap-2"
                                            >
                                                <Send className="w-4 h-4" />
                                                {isSubmitting ? 'Sending...' : 'Send Reset Email'}
                                            </button>
                                        </div>
                                    </div>
                                )}
                            </div>
                        )}

                        {/* Change Email Action */}
                        {canResetPassword && (
                            <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
                                <button
                                    onClick={() => toggleAction('email')}
                                    className="w-full p-4 flex items-center justify-between text-left hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
                                >
                                    <div className="flex items-center gap-3">
                                        <Mail className="w-5 h-5 text-purple-600 dark:text-purple-400" />
                                        <div>
                                            <p className="font-medium text-gray-900 dark:text-white">Change Email</p>
                                            <p className="text-xs text-gray-500 dark:text-gray-400">
                                                Update the user's email address
                                            </p>
                                        </div>
                                    </div>
                                    {activeAction === 'email' 
                                        ? <ChevronUp className="w-5 h-5 text-gray-400" />
                                        : <ChevronDown className="w-5 h-5 text-gray-400" />
                                    }
                                </button>

                                {activeAction === 'email' && (
                                    <div className="p-4 bg-gray-50 dark:bg-gray-700/30 border-t border-gray-200 dark:border-gray-700 space-y-3">
                                        <div>
                                            <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                                                Current Email
                                            </label>
                                            <p className="text-sm text-gray-500 dark:text-gray-400 font-mono bg-gray-100 dark:bg-gray-800 px-3 py-2 rounded-md">
                                                {user?.email}
                                            </p>
                                        </div>
                                        <div>
                                            <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                                                New Email
                                            </label>
                                            <input
                                                type="email"
                                                value={newEmail}
                                                onChange={(e) => setNewEmail(e.target.value)}
                                                placeholder="Enter new email address"
                                                className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                            />
                                        </div>
                                        <button
                                            onClick={handleChangeEmail}
                                            disabled={isSubmitting || !newEmail || !newEmail.includes('@')}
                                            className="w-full px-4 py-2 bg-purple-600 text-white rounded-lg text-sm font-medium hover:bg-purple-700 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                                        >
                                            <Mail className="w-4 h-4" />
                                            {isSubmitting ? 'Updating...' : 'Update Email'}
                                        </button>
                                    </div>
                                )}
                            </div>
                        )}

                        {/* Suspend/Unsuspend Action */}
                        {canSuspend && (
                            <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
                                <button
                                    onClick={() => isUserSuspended ? handleUnsuspend() : toggleAction('suspend')}
                                    disabled={isSubmitting}
                                    className={clsx(
                                        "w-full p-4 flex items-center justify-between text-left transition-colors",
                                        isUserSuspended 
                                            ? "bg-green-50 dark:bg-green-900/20 hover:bg-green-100 dark:hover:bg-green-900/30"
                                            : "hover:bg-gray-50 dark:hover:bg-gray-700/50"
                                    )}
                                >
                                    <div className="flex items-center gap-3">
                                        {isUserSuspended ? (
                                            <UserCheck className="w-5 h-5 text-green-600 dark:text-green-400" />
                                        ) : (
                                            <Ban className="w-5 h-5 text-amber-600 dark:text-amber-400" />
                                        )}
                                        <div>
                                            <p className="font-medium text-gray-900 dark:text-white">
                                                {isUserSuspended ? 'Unsuspend User' : 'Suspend User'}
                                            </p>
                                            <p className="text-xs text-gray-500 dark:text-gray-400">
                                                {isUserSuspended 
                                                    ? 'Restore access to this account' 
                                                    : 'Block access temporarily or indefinitely'}
                                            </p>
                                        </div>
                                    </div>
                                    {!isUserSuspended && (
                                        activeAction === 'suspend' 
                                            ? <ChevronUp className="w-5 h-5 text-gray-400" />
                                            : <ChevronDown className="w-5 h-5 text-gray-400" />
                                    )}
                                </button>

                                {/* Suspend Form */}
                                {activeAction === 'suspend' && !isUserSuspended && (
                                    <div className="p-4 bg-gray-50 dark:bg-gray-700/30 border-t border-gray-200 dark:border-gray-700 space-y-4">
                                        <div className="space-y-2">
                                            <label className="flex items-center gap-3 p-3 border border-gray-200 dark:border-gray-600 rounded-lg cursor-pointer hover:bg-white dark:hover:bg-gray-700/50 transition-colors bg-white dark:bg-gray-800">
                                                <input
                                                    type="radio"
                                                    checked={suspensionType === 'indefinite'}
                                                    onChange={() => setSuspensionType('indefinite')}
                                                    className="text-primary-600 focus:ring-primary-500"
                                                />
                                                <div>
                                                    <p className="text-sm font-medium text-gray-900 dark:text-white">Indefinite</p>
                                                    <p className="text-xs text-gray-500 dark:text-gray-400">Until manually unsuspended</p>
                                                </div>
                                            </label>
                                            
                                            <label className="flex items-start gap-3 p-3 border border-gray-200 dark:border-gray-600 rounded-lg cursor-pointer hover:bg-white dark:hover:bg-gray-700/50 transition-colors bg-white dark:bg-gray-800">
                                                <input
                                                    type="radio"
                                                    checked={suspensionType === 'timed'}
                                                    onChange={() => setSuspensionType('timed')}
                                                    className="mt-1 text-primary-600 focus:ring-primary-500"
                                                />
                                                <div className="flex-1">
                                                    <p className="text-sm font-medium text-gray-900 dark:text-white">Until specific date</p>
                                                    {suspensionType === 'timed' && (
                                                        <div className="flex gap-2 mt-2">
                                                            <div className="flex-1">
                                                                <label className="block text-xs text-gray-500 mb-1">
                                                                    <Calendar className="w-3 h-3 inline mr-1" />
                                                                    End Date
                                                                </label>
                                                                <input
                                                                    type="date"
                                                                    min={today}
                                                                    value={untilDate}
                                                                    onChange={(e) => setUntilDate(e.target.value)}
                                                                    className="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                                                />
                                                            </div>
                                                            <div className="w-24">
                                                                <label className="block text-xs text-gray-500 mb-1">
                                                                    <Clock className="w-3 h-3 inline mr-1" />
                                                                    Time
                                                                </label>
                                                                <input
                                                                    type="time"
                                                                    value={untilTime}
                                                                    onChange={(e) => setUntilTime(e.target.value)}
                                                                    className="w-full px-2 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                                                />
                                                            </div>
                                                        </div>
                                                    )}
                                                </div>
                                            </label>
                                        </div>

                                        <div>
                                            <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                                                Reason (Optional)
                                            </label>
                                            <textarea
                                                value={suspendReason}
                                                onChange={(e) => setSuspendReason(e.target.value)}
                                                placeholder="Enter a reason for the suspension"
                                                rows={2}
                                                className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-md bg-white dark:bg-gray-700 text-gray-900 dark:text-white resize-none"
                                            />
                                        </div>

                                        <button
                                            onClick={handleSuspend}
                                            disabled={isSubmitting}
                                            className="w-full px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50"
                                        >
                                            {isSubmitting ? 'Suspending...' : 'Suspend User'}
                                        </button>
                                    </div>
                                )}
                            </div>
                        )}

                        {/* Permanent Delete Action */}
                        {canDelete && (
                            <div className="border border-red-200 dark:border-red-800 rounded-lg overflow-hidden">
                                <button
                                    onClick={() => toggleAction('delete')}
                                    className="w-full p-4 flex items-center justify-between text-left hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
                                >
                                    <div className="flex items-center gap-3">
                                        <Trash2 className="w-5 h-5 text-red-600 dark:text-red-400" />
                                        <div>
                                            <p className="font-medium text-red-700 dark:text-red-400">Permanently Delete</p>
                                            <p className="text-xs text-red-600 dark:text-red-400">
                                                Cannot be undone
                                            </p>
                                        </div>
                                    </div>
                                    {activeAction === 'delete' 
                                        ? <ChevronUp className="w-5 h-5 text-red-400" />
                                        : <ChevronDown className="w-5 h-5 text-red-400" />
                                    }
                                </button>

                                {activeAction === 'delete' && (
                                    <div className="p-4 bg-red-50 dark:bg-red-900/20 border-t border-red-200 dark:border-red-800 space-y-3">
                                        <div className="flex items-start gap-2 text-sm text-red-800 dark:text-red-200">
                                            <AlertTriangle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                                            <p>
                                                <strong>Warning:</strong> This action cannot be undone. The user and all their personal data will be permanently removed from the system.
                                            </p>
                                        </div>
                                        
                                        <div>
                                            <label className="block text-xs font-medium text-red-700 dark:text-red-300 mb-1">
                                                Type <span className="font-mono bg-red-100 dark:bg-red-900/50 px-1 rounded">{user.email}</span> to confirm
                                            </label>
                                            <input
                                                type="text"
                                                value={confirmEmail}
                                                onChange={(e) => setConfirmEmail(e.target.value)}
                                                placeholder={user.email}
                                                className="w-full px-3 py-2 text-sm border border-red-300 dark:border-red-700 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white font-mono"
                                            />
                                        </div>

                                        <button
                                            onClick={handlePermanentDelete}
                                            disabled={isSubmitting || confirmEmail !== user.email}
                                            className="w-full px-4 py-2 bg-red-600 text-white rounded-lg text-sm font-medium hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed"
                                        >
                                            {isSubmitting ? 'Deleting...' : 'Permanently Delete User'}
                                        </button>
                                    </div>
                                )}
                            </div>
                        )}
                    </div>
                </div>

                {/* Footer */}
                <div className="p-4 border-t border-gray-200 dark:border-gray-700 flex-shrink-0">
                    <button
                        onClick={handleClose}
                        className="w-full px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-200 bg-gray-100 dark:bg-gray-700 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
                    >
                        Close
                    </button>
                </div>
            </div>
        </div>
    );
}
