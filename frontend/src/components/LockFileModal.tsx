import { useState } from 'react';
import { X, Lock, Eye, EyeOff, Shield, Info } from 'lucide-react';
import clsx from 'clsx';

interface LockFileModalProps {
    isOpen: boolean;
    onClose: () => void;
    onLock: (password: string | null, requiredRole: string | null) => Promise<void>;
    fileName: string;
    isLocking: boolean;
}

const ROLE_OPTIONS = [
    { value: '', label: 'No role requirement', description: 'Anyone with basic unlock permission can unlock' },
    { value: 'Employee', label: 'Employee or higher', description: 'Employee, Manager, Admin, or SuperAdmin can unlock' },
    { value: 'Manager', label: 'Manager or higher', description: 'Manager, Admin, or SuperAdmin can unlock' },
    { value: 'Admin', label: 'Admin or higher', description: 'Admin or SuperAdmin can unlock' },
];

export function LockFileModal({ isOpen, onClose, onLock, fileName, isLocking }: LockFileModalProps) {
    const [usePassword, setUsePassword] = useState(false);
    const [password, setPassword] = useState('');
    const [confirmPassword, setConfirmPassword] = useState('');
    const [showPassword, setShowPassword] = useState(false);
    const [requiredRole, setRequiredRole] = useState('');
    const [error, setError] = useState('');

    if (!isOpen) return null;

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setError('');

        if (usePassword) {
            if (password.length < 4) {
                setError('Password must be at least 4 characters');
                return;
            }
            if (password !== confirmPassword) {
                setError('Passwords do not match');
                return;
            }
        }

        try {
            await onLock(
                usePassword && password ? password : null,
                requiredRole || null
            );
            // Reset form
            setPassword('');
            setConfirmPassword('');
            setUsePassword(false);
            setRequiredRole('');
            onClose();
        } catch (err) {
            setError('Failed to lock file');
        }
    };

    const handleClose = () => {
        setPassword('');
        setConfirmPassword('');
        setUsePassword(false);
        setRequiredRole('');
        setError('');
        onClose();
    };

    return (
        <div className="fixed inset-0 z-50 overflow-y-auto">
            <div className="flex items-center justify-center min-h-screen px-4">
                <div className="fixed inset-0 bg-black/50 transition-opacity" onClick={handleClose} />
                
                <div className="relative bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full p-6">
                    <div className="flex items-center justify-between mb-4">
                        <div className="flex items-center gap-3">
                            <div className="p-2 bg-yellow-100 dark:bg-yellow-900/30 rounded-lg">
                                <Lock className="w-5 h-5 text-yellow-600 dark:text-yellow-400" />
                            </div>
                            <div>
                                <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                                    Lock File
                                </h3>
                                <p className="text-sm text-gray-500 dark:text-gray-400 truncate max-w-[250px]">
                                    {fileName}
                                </p>
                            </div>
                        </div>
                        <button onClick={handleClose} className="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded">
                            <X className="w-5 h-5 text-gray-500" />
                        </button>
                    </div>

                    <form onSubmit={handleSubmit} className="space-y-4">
                        {/* Role Requirement */}
                        <div>
                            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                <Shield className="w-4 h-4 inline mr-1" />
                                Minimum Role to Unlock
                            </label>
                            <select
                                value={requiredRole}
                                onChange={(e) => setRequiredRole(e.target.value)}
                                className="w-full px-3 py-2 border border-gray-200 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500"
                            >
                                {ROLE_OPTIONS.map((opt) => (
                                    <option key={opt.value} value={opt.value}>
                                        {opt.label}
                                    </option>
                                ))}
                            </select>
                            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                                {ROLE_OPTIONS.find(o => o.value === requiredRole)?.description}
                            </p>
                        </div>

                        {/* Password Toggle */}
                        <div className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                            <div>
                                <p className="text-sm font-medium text-gray-700 dark:text-gray-300">
                                    Require Password
                                </p>
                                <p className="text-xs text-gray-500 dark:text-gray-400">
                                    Additional security for unlocking
                                </p>
                            </div>
                            <label className="relative inline-flex items-center cursor-pointer">
                                <input
                                    type="checkbox"
                                    checked={usePassword}
                                    onChange={(e) => setUsePassword(e.target.checked)}
                                    className="sr-only peer"
                                />
                                <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-primary-300 dark:peer-focus:ring-primary-800 rounded-full peer dark:bg-gray-600 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-500 peer-checked:bg-primary-600"></div>
                            </label>
                        </div>

                        {/* Password Fields */}
                        {usePassword && (
                            <div className="space-y-3">
                                <div>
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                        Password
                                    </label>
                                    <div className="relative">
                                        <input
                                            type={showPassword ? "text" : "password"}
                                            value={password}
                                            onChange={(e) => setPassword(e.target.value)}
                                            placeholder="Enter lock password"
                                            className="w-full px-3 py-2 pr-10 border border-gray-200 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500"
                                        />
                                        <button
                                            type="button"
                                            onClick={() => setShowPassword(!showPassword)}
                                            className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-gray-400 hover:text-gray-600"
                                        >
                                            {showPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                                        </button>
                                    </div>
                                </div>
                                <div>
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                        Confirm Password
                                    </label>
                                    <input
                                        type={showPassword ? "text" : "password"}
                                        value={confirmPassword}
                                        onChange={(e) => setConfirmPassword(e.target.value)}
                                        placeholder="Confirm password"
                                        className="w-full px-3 py-2 border border-gray-200 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500"
                                    />
                                </div>
                            </div>
                        )}

                        {error && (
                            <div className="p-3 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300 rounded-lg text-sm">
                                {error}
                            </div>
                        )}

                        {/* Info */}
                        <div className="p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
                            <div className="flex gap-2">
                                <Info className="w-4 h-4 text-blue-500 mt-0.5 flex-shrink-0" />
                                <p className="text-xs text-blue-700 dark:text-blue-300">
                                    The file owner can always unlock their own files. SuperAdmins can unlock any file.
                                </p>
                            </div>
                        </div>

                        {/* Actions */}
                        <div className="flex gap-3 pt-2">
                            <button
                                type="button"
                                onClick={handleClose}
                                className="flex-1 px-4 py-2 border border-gray-200 dark:border-gray-600 text-gray-700 dark:text-gray-200 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
                            >
                                Cancel
                            </button>
                            <button
                                type="submit"
                                disabled={isLocking}
                                className={clsx(
                                    "flex-1 px-4 py-2 bg-yellow-600 text-white rounded-lg font-medium",
                                    isLocking ? "opacity-50 cursor-not-allowed" : "hover:bg-yellow-700"
                                )}
                            >
                                {isLocking ? 'Locking...' : 'Lock File'}
                            </button>
                        </div>
                    </form>
                </div>
            </div>
        </div>
    );
}

// Unlock modal for password-protected files
interface UnlockFileModalProps {
    isOpen: boolean;
    onClose: () => void;
    onUnlock: (password: string | null) => Promise<{ error?: string; requires_password?: boolean }>;
    fileName: string;
    isUnlocking: boolean;
    requiresPassword: boolean;
    requiredRole?: string;
}

export function UnlockFileModal({ 
    isOpen, 
    onClose, 
    onUnlock, 
    fileName, 
    isUnlocking,
    requiresPassword,
    requiredRole
}: UnlockFileModalProps) {
    const [password, setPassword] = useState('');
    const [showPassword, setShowPassword] = useState(false);
    const [error, setError] = useState('');

    if (!isOpen) return null;

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setError('');

        if (requiresPassword && !password) {
            setError('Password is required');
            return;
        }

        try {
            const result = await onUnlock(requiresPassword ? password : null);
            if (result.error) {
                setError(result.error);
            } else {
                setPassword('');
                onClose();
            }
        } catch (err) {
            setError('Failed to unlock file');
        }
    };

    const handleClose = () => {
        setPassword('');
        setError('');
        onClose();
    };

    return (
        <div className="fixed inset-0 z-50 overflow-y-auto">
            <div className="flex items-center justify-center min-h-screen px-4">
                <div className="fixed inset-0 bg-black/50 transition-opacity" onClick={handleClose} />
                
                <div className="relative bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full p-6">
                    <div className="flex items-center justify-between mb-4">
                        <div className="flex items-center gap-3">
                            <div className="p-2 bg-green-100 dark:bg-green-900/30 rounded-lg">
                                <Lock className="w-5 h-5 text-green-600 dark:text-green-400" />
                            </div>
                            <div>
                                <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                                    Unlock File
                                </h3>
                                <p className="text-sm text-gray-500 dark:text-gray-400 truncate max-w-[250px]">
                                    {fileName}
                                </p>
                            </div>
                        </div>
                        <button onClick={handleClose} className="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded">
                            <X className="w-5 h-5 text-gray-500" />
                        </button>
                    </div>

                    <form onSubmit={handleSubmit} className="space-y-4">
                        {requiredRole && (
                            <div className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                                <p className="text-sm text-gray-600 dark:text-gray-300">
                                    <Shield className="w-4 h-4 inline mr-1" />
                                    Required role: <span className="font-medium">{requiredRole}</span> or higher
                                </p>
                            </div>
                        )}

                        {requiresPassword && (
                            <div>
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    Password
                                </label>
                                <div className="relative">
                                    <input
                                        type={showPassword ? "text" : "password"}
                                        value={password}
                                        onChange={(e) => setPassword(e.target.value)}
                                        placeholder="Enter unlock password"
                                        className="w-full px-3 py-2 pr-10 border border-gray-200 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500"
                                        autoFocus
                                    />
                                    <button
                                        type="button"
                                        onClick={() => setShowPassword(!showPassword)}
                                        className="absolute right-2 top-1/2 -translate-y-1/2 p-1 text-gray-400 hover:text-gray-600"
                                    >
                                        {showPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                                    </button>
                                </div>
                            </div>
                        )}

                        {error && (
                            <div className="p-3 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300 rounded-lg text-sm">
                                {error}
                            </div>
                        )}

                        {/* Actions */}
                        <div className="flex gap-3 pt-2">
                            <button
                                type="button"
                                onClick={handleClose}
                                className="flex-1 px-4 py-2 border border-gray-200 dark:border-gray-600 text-gray-700 dark:text-gray-200 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700"
                            >
                                Cancel
                            </button>
                            <button
                                type="submit"
                                disabled={isUnlocking}
                                className={clsx(
                                    "flex-1 px-4 py-2 bg-green-600 text-white rounded-lg font-medium",
                                    isUnlocking ? "opacity-50 cursor-not-allowed" : "hover:bg-green-700"
                                )}
                            >
                                {isUnlocking ? 'Unlocking...' : 'Unlock File'}
                            </button>
                        </div>
                    </form>
                </div>
            </div>
        </div>
    );
}
