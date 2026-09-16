import { useState, useEffect, useMemo } from 'react';
import { X, UserPlus, Search, UserCheck, Shield, Building2, Key, RefreshCw, Eye, EyeOff, Check, AlertCircle, Loader2 } from 'lucide-react';
import clsx from 'clsx';
import { useAuthFetch } from '../context/AuthContext';
import { useTranslations } from '../context/I18nContext';

interface AddMemberModalProps {
    isOpen: boolean;
    onClose: () => void;
    companyId: string;
    companyName: string;
    existingUserIds: string[];
    onSuccess: () => void;
    departments: { id: string; name: string }[];
}

interface SystemUser {
    id: string;
    name: string;
    email: string;
    role: string;
    status: string;
    tenant_id: string;
    allowed_tenant_ids?: string[] | null;
}

const generatePassword = (length = 12): string => {
    const lowercase = 'abcdefghijklmnopqrstuvwxyz';
    const uppercase = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';
    const numbers = '0123456789';
    const symbols = '!@#$%^&*';
    const allChars = lowercase + uppercase + numbers + symbols;
    let password = '';
    password += lowercase[Math.floor(Math.random() * lowercase.length)];
    password += uppercase[Math.floor(Math.random() * uppercase.length)];
    password += numbers[Math.floor(Math.random() * numbers.length)];
    password += symbols[Math.floor(Math.random() * symbols.length)];
    for (let i = 4; i < length; i++) {
        password += allChars[Math.floor(Math.random() * allChars.length)];
    }
    return password.split('').sort(() => Math.random() - 0.5).join('');
};

export function AddMemberModal({
    isOpen,
    onClose,
    companyId,
    companyName,
    existingUserIds,
    onSuccess,
    departments,
}: AddMemberModalProps) {
    const t = useTranslations('CompanyDetails');
    const tUsers = useTranslations('Users');
    const tCommon = useTranslations('Common');
    const authFetch = useAuthFetch();

    const [activeTab, setActiveTab] = useState<'existing' | 'create'>('existing');

    // Tab 1: Existing users state
    const [allUsers, setAllUsers] = useState<SystemUser[]>([]);
    const [isLoadingUsers, setIsLoadingUsers] = useState(false);
    const [searchQuery, setSearchQuery] = useState('');
    const [selectedUserId, setSelectedUserId] = useState<string | null>(null);

    // Tab 2: Create new user state
    const [newName, setNewName] = useState('');
    const [newEmail, setNewEmail] = useState('');
    const [newRole, setNewRole] = useState('Employee');
    const [newDeptId, setNewDeptId] = useState('');
    const [newPassword, setNewPassword] = useState('');
    const [showPassword, setShowPassword] = useState(false);

    // Status state
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Fetch system users when modal opens
    useEffect(() => {
        if (isOpen) {
            setError(null);
            setSelectedUserId(null);
            setSearchQuery('');
            setNewName('');
            setNewEmail('');
            setNewRole('Employee');
            setNewDeptId('');
            setNewPassword(generatePassword());
            setShowPassword(false);
            fetchSystemUsers();
        }
    }, [isOpen, companyId]);

    const fetchSystemUsers = async () => {
        setIsLoadingUsers(true);
        try {
            const res = await authFetch('/api/users');
            if (res.ok) {
                const data: SystemUser[] = await res.json();
                setAllUsers(data);
            }
        } catch (err) {
            console.error('Failed to fetch system users', err);
        } finally {
            setIsLoadingUsers(false);
        }
    };

    // Filter users not already in this company
    const availableUsers = useMemo(() => {
        return allUsers.filter(user => {
            const isPrimary = user.tenant_id === companyId;
            const isAllowed = user.allowed_tenant_ids?.includes(companyId);
            return !isPrimary && !isAllowed && !existingUserIds.includes(user.id);
        });
    }, [allUsers, companyId, existingUserIds]);

    // Search filtered
    const searchedUsers = useMemo(() => {
        if (!searchQuery.trim()) return availableUsers;
        const q = searchQuery.toLowerCase();
        return availableUsers.filter(
            u => u.name.toLowerCase().includes(q) || u.email.toLowerCase().includes(q)
        );
    }, [availableUsers, searchQuery]);

    // Submit Tab 1: Add existing user to this company
    const handleAddExisting = async () => {
        if (!selectedUserId) return;
        setIsSubmitting(true);
        setError(null);
        try {
            const targetUser = allUsers.find(u => u.id === selectedUserId);
            if (!targetUser) throw new Error('User not found');

            const currentAllowed = targetUser.allowed_tenant_ids || [];
            const newAllowed = Array.from(new Set([...currentAllowed, companyId]));

            const res = await authFetch(`/api/users/${selectedUserId}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    allowed_tenant_ids: newAllowed,
                }),
            });

            if (res.ok) {
                onSuccess();
                onClose();
            } else {
                const errData = await res.json().catch(() => ({}));
                setError(errData.message || 'Failed to add user to company');
            }
        } catch (err: any) {
            setError(err.message || 'An error occurred');
        } finally {
            setIsSubmitting(false);
        }
    };

    // Submit Tab 2: Create new user directly in this company
    const handleCreateNew = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!newName.trim() || !newEmail.trim() || !newPassword) {
            setError('请完整填写所有必填项');
            return;
        }

        if (newPassword.length < 8) {
            setError('密码长度至少需要 8 位字符');
            return;
        }

        if (!/[A-Z]/.test(newPassword)) {
            setError('密码必须包含至少一个大写字母 (A-Z)');
            return;
        }

        if (!/[a-z]/.test(newPassword)) {
            setError('密码必须包含至少一个小写字母 (a-z)');
            return;
        }

        if (!/[0-9]/.test(newPassword)) {
            setError('密码必须包含至少一个数字 (0-9)');
            return;
        }

        setIsSubmitting(true);
        setError(null);
        try {
            const res = await authFetch('/api/users', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    name: newName.trim(),
                    email: newEmail.trim(),
                    password: newPassword,
                    role: newRole,
                    department_id: newDeptId || null,
                    tenant_id: companyId,
                }),
            });

            if (res.ok) {
                onSuccess();
                onClose();
            } else {
                const errData = await res.json().catch(() => ({}));
                const errMsg = errData.message 
                    || (Array.isArray(errData.requirements) ? errData.requirements.join('; ') : null)
                    || errData.error 
                    || '创建用户失败，请检查填写内容';
                setError(errMsg);
            }
        } catch (err: any) {
            setError(err.message || 'An error occurred');
        } finally {
            setIsSubmitting(false);
        }
    };

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm animate-in fade-in duration-150">
            <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl border border-gray-200 dark:border-gray-700 w-full max-w-xl overflow-hidden flex flex-col max-h-[90vh]">
                {/* Modal Header */}
                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between bg-gray-50/50 dark:bg-gray-900/30">
                    <div className="flex items-center gap-3">
                        <div className="p-2 rounded-xl bg-primary-100 dark:bg-primary-900/40 text-primary-600 dark:text-primary-400">
                            <UserPlus className="w-5 h-5" />
                        </div>
                        <div>
                            <h3 className="text-base font-semibold text-gray-900 dark:text-white flex items-center gap-2">
                                <span>{t('addMember') || '添加企业成员'}</span>
                                <span className="text-xs font-medium px-2 py-0.5 rounded-md bg-primary-50 dark:bg-primary-900/20 text-primary-700 dark:text-primary-300 border border-primary-200 dark:border-primary-800">
                                    {companyName}
                                </span>
                            </h3>
                            <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                                {activeTab === 'existing'
                                    ? (t('addExistingUserDesc') || '从系统现有账号中挑选成员加入此企业空间')
                                    : (t('createNewMemberDesc') || '直接创建属于此企业的新账号并分配权限')}
                            </p>
                        </div>
                    </div>
                    <button
                        onClick={onClose}
                        className="p-1.5 rounded-lg text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <X className="w-5 h-5" />
                    </button>
                </div>

                {/* Tabs */}
                <div className="px-6 pt-3 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 flex gap-6">
                    <button
                        type="button"
                        onClick={() => { setActiveTab('existing'); setError(null); }}
                        className={clsx(
                            'pb-3 text-sm font-medium border-b-2 transition-colors flex items-center gap-2',
                            activeTab === 'existing'
                                ? 'border-primary-600 text-primary-600 dark:text-primary-400 font-semibold'
                                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400'
                        )}
                    >
                        <UserCheck className="w-4 h-4" />
                        <span>{t('addExistingUser') || '从系统已有用户添加'}</span>
                        <span className="ml-1 px-1.5 py-0.5 text-xs rounded-full bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300">
                            {availableUsers.length}
                        </span>
                    </button>
                    <button
                        type="button"
                        onClick={() => { setActiveTab('create'); setError(null); }}
                        className={clsx(
                            'pb-3 text-sm font-medium border-b-2 transition-colors flex items-center gap-2',
                            activeTab === 'create'
                                ? 'border-primary-600 text-primary-600 dark:text-primary-400 font-semibold'
                                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400'
                        )}
                    >
                        <UserPlus className="w-4 h-4" />
                        <span>{t('createNewMember') || '直接创建新成员'}</span>
                    </button>
                </div>

                {/* Error Banner */}
                {error && (
                    <div className="mx-6 mt-4 p-3 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 flex items-center gap-2 text-xs text-red-600 dark:text-red-400">
                        <AlertCircle className="w-4 h-4 shrink-0" />
                        <span>{error}</span>
                    </div>
                )}

                {/* Tab 1: Existing Users */}
                {activeTab === 'existing' && (
                    <div className="p-6 space-y-4 flex-1 overflow-y-auto">
                        {/* Search Input */}
                        <div className="relative">
                            <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                            <input
                                type="text"
                                value={searchQuery}
                                onChange={(e) => setSearchQuery(e.target.value)}
                                placeholder={t('searchUsersPlaceholder') || '搜索姓名或邮箱...'}
                                className="w-full pl-9 pr-4 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-400 focus:outline-none focus:ring-2 focus:ring-primary-500"
                            />
                        </div>

                        {/* User List */}
                        <div className="border border-gray-200 dark:border-gray-700 rounded-xl overflow-hidden divide-y divide-gray-100 dark:divide-gray-700 max-h-64 overflow-y-auto">
                            {isLoadingUsers ? (
                                <div className="p-8 text-center text-gray-500 flex items-center justify-center gap-2 text-sm">
                                    <Loader2 className="w-4 h-4 animate-spin" />
                                    <span>{tCommon('loading') || '加载用户列表中...'}</span>
                                </div>
                            ) : searchedUsers.length === 0 ? (
                                <div className="p-8 text-center text-gray-500 dark:text-gray-400 text-xs">
                                    {availableUsers.length === 0
                                        ? (t('noAvailableUsers') || '暂无可添加的用户（系统所有用户已在此企业中）')
                                        : (t('noSearchResults') || '未搜索到匹配的用户')}
                                </div>
                            ) : (
                                searchedUsers.map(user => {
                                    const isSelected = selectedUserId === user.id;
                                    return (
                                        <div
                                            key={user.id}
                                            onClick={() => setSelectedUserId(user.id)}
                                            className={clsx(
                                                'p-3 flex items-center justify-between cursor-pointer transition-colors text-left',
                                                isSelected
                                                    ? 'bg-primary-50/70 dark:bg-primary-900/30'
                                                    : 'hover:bg-gray-50 dark:hover:bg-gray-700/50'
                                            )}
                                        >
                                            <div className="flex items-center gap-3 min-w-0">
                                                <div className={clsx(
                                                    'w-8 h-8 rounded-full flex items-center justify-center font-medium text-xs shrink-0',
                                                    isSelected
                                                        ? 'bg-primary-600 text-white'
                                                        : 'bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300'
                                                )}>
                                                    {user.name.charAt(0).toUpperCase()}
                                                </div>
                                                <div className="min-w-0">
                                                    <p className="text-xs font-semibold text-gray-900 dark:text-white truncate">
                                                        {user.name}
                                                    </p>
                                                    <p className="text-[11px] text-gray-500 dark:text-gray-400 truncate">
                                                        {user.email}
                                                    </p>
                                                </div>
                                            </div>
                                            <div className="flex items-center gap-2 shrink-0">
                                                <span className="px-2 py-0.5 text-[10px] font-medium rounded-md bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300">
                                                    {user.role}
                                                </span>
                                                <div className={clsx(
                                                    'w-4 h-4 rounded-full border flex items-center justify-center transition-colors',
                                                    isSelected
                                                        ? 'border-primary-600 bg-primary-600 text-white'
                                                        : 'border-gray-300 dark:border-gray-600'
                                                )}>
                                                    {isSelected && <Check className="w-3 h-3 stroke-[3]" />}
                                                </div>
                                            </div>
                                        </div>
                                    );
                                })
                            )}
                        </div>
                    </div>
                )}

                {/* Tab 2: Create New User */}
                {activeTab === 'create' && (
                    <form onSubmit={handleCreateNew} className="p-6 space-y-4 flex-1 overflow-y-auto">
                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                            <div>
                                <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    {tUsers('name') || '姓名'} <span className="text-red-500">*</span>
                                </label>
                                <input
                                    type="text"
                                    required
                                    value={newName}
                                    onChange={(e) => setNewName(e.target.value)}
                                    placeholder="例如：张三"
                                    className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 outline-none"
                                />
                            </div>

                            <div>
                                <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    {tUsers('email') || '登录邮箱'} <span className="text-red-500">*</span>
                                </label>
                                <input
                                    type="email"
                                    required
                                    value={newEmail}
                                    onChange={(e) => setNewEmail(e.target.value)}
                                    placeholder="zhangsan@company.com"
                                    className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 outline-none"
                                />
                            </div>
                        </div>

                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                            <div>
                                <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    {tUsers('role') || '角色权限'}
                                </label>
                                <select
                                    value={newRole}
                                    onChange={(e) => setNewRole(e.target.value)}
                                    className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 outline-none"
                                >
                                    <option value="Employee">{tUsers('roleEmployee') || '普通员工 (Employee)'}</option>
                                    <option value="Manager">{tUsers('roleManager') || '部门主管 (Manager)'}</option>
                                    <option value="Admin">{tUsers('roleAdmin') || '管理员 (Admin)'}</option>
                                </select>
                            </div>

                            <div>
                                <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    {tUsers('department') || '所属部门'}
                                </label>
                                <select
                                    value={newDeptId}
                                    onChange={(e) => setNewDeptId(e.target.value)}
                                    className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 outline-none"
                                >
                                    <option value="">{t('unassignedDept') || '未分配部门'}</option>
                                    {departments.map(dept => (
                                        <option key={dept.id} value={dept.id}>{dept.name}</option>
                                    ))}
                                </select>
                            </div>
                        </div>

                        <div>
                            <div className="flex items-center justify-between mb-1">
                                <label className="block text-xs font-medium text-gray-700 dark:text-gray-300">
                                    {tUsers('password') || '初始密码'} <span className="text-red-500">*</span>
                                </label>
                                <button
                                    type="button"
                                    onClick={() => setNewPassword(generatePassword())}
                                    className="text-xs text-primary-600 hover:text-primary-700 dark:text-primary-400 flex items-center gap-1"
                                >
                                    <RefreshCw className="w-3 h-3" />
                                    <span>{tUsers('generatePassword') || '随机生成'}</span>
                                </button>
                            </div>
                            <div className="relative">
                                <input
                                    type={showPassword ? 'text' : 'password'}
                                    required
                                    value={newPassword}
                                    onChange={(e) => setNewPassword(e.target.value)}
                                    placeholder="输入初始登录密码"
                                    className="w-full pl-3 pr-10 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 outline-none font-mono"
                                />
                                <button
                                    type="button"
                                    onClick={() => setShowPassword(!showPassword)}
                                    className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                                >
                                    {showPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                                </button>
                            </div>

                            {/* Password requirements checklist */}
                            <div className="mt-2.5 p-2.5 rounded-lg bg-gray-50 dark:bg-gray-700/50 border border-gray-200/70 dark:border-gray-600/50 text-xs">
                                <div className="text-gray-500 dark:text-gray-400 font-medium mb-1.5 flex items-center justify-between">
                                    <span>密码安全策略要求：</span>
                                    {newPassword.length >= 8 && /[A-Z]/.test(newPassword) && /[a-z]/.test(newPassword) && /[0-9]/.test(newPassword) && (
                                        <span className="text-green-600 dark:text-green-400 font-semibold flex items-center gap-1 text-[11px]">
                                            <Check className="w-3.5 h-3.5" /> 符合策略
                                        </span>
                                    )}
                                </div>
                                <div className="grid grid-cols-2 gap-1.5 text-[11px]">
                                    <div className={clsx("flex items-center gap-1.5 transition-colors", newPassword.length >= 8 ? "text-green-600 dark:text-green-400 font-medium" : "text-gray-500 dark:text-gray-400")}>
                                        <span className={clsx("w-1.5 h-1.5 rounded-full shrink-0", newPassword.length >= 8 ? "bg-green-500" : "bg-gray-300 dark:bg-gray-500")}></span>
                                        至少 8 位字符
                                    </div>
                                    <div className={clsx("flex items-center gap-1.5 transition-colors", /[A-Z]/.test(newPassword) ? "text-green-600 dark:text-green-400 font-medium" : "text-amber-600 dark:text-amber-400 font-medium")}>
                                        <span className={clsx("w-1.5 h-1.5 rounded-full shrink-0", /[A-Z]/.test(newPassword) ? "bg-green-500" : "bg-amber-400")}></span>
                                        含大写字母 (A-Z)
                                    </div>
                                    <div className={clsx("flex items-center gap-1.5 transition-colors", /[a-z]/.test(newPassword) ? "text-green-600 dark:text-green-400 font-medium" : "text-gray-500 dark:text-gray-400")}>
                                        <span className={clsx("w-1.5 h-1.5 rounded-full shrink-0", /[a-z]/.test(newPassword) ? "bg-green-500" : "bg-gray-300 dark:bg-gray-500")}></span>
                                        含小写字母 (a-z)
                                    </div>
                                    <div className={clsx("flex items-center gap-1.5 transition-colors", /[0-9]/.test(newPassword) ? "text-green-600 dark:text-green-400 font-medium" : "text-gray-500 dark:text-gray-400")}>
                                        <span className={clsx("w-1.5 h-1.5 rounded-full shrink-0", /[0-9]/.test(newPassword) ? "bg-green-500" : "bg-gray-300 dark:bg-gray-500")}></span>
                                        含数字 (0-9)
                                    </div>
                                </div>
                            </div>
                        </div>
                    </form>
                )}

                {/* Modal Footer */}
                <div className="px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50/50 dark:bg-gray-900/30 flex items-center justify-end gap-3">
                    <button
                        type="button"
                        onClick={onClose}
                        className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                    >
                        {tCommon('cancel') || '取消'}
                    </button>

                    {activeTab === 'existing' ? (
                        <button
                            type="button"
                            onClick={handleAddExisting}
                            disabled={!selectedUserId || isSubmitting}
                            className={clsx(
                                'flex items-center px-4 py-2 text-sm font-medium rounded-lg text-white transition-all shadow-sm',
                                selectedUserId && !isSubmitting
                                    ? 'bg-primary-600 hover:bg-primary-700 cursor-pointer'
                                    : 'bg-gray-300 dark:bg-gray-700 text-gray-400 cursor-not-allowed'
                            )}
                        >
                            {isSubmitting && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                            <span>{t('confirmAddMember') || '添加到此企业'}</span>
                        </button>
                    ) : (
                        <button
                            type="button"
                            onClick={handleCreateNew}
                            disabled={!newName.trim() || !newEmail.trim() || !newPassword || isSubmitting}
                            className={clsx(
                                'flex items-center px-4 py-2 text-sm font-medium rounded-lg text-white transition-all shadow-sm',
                                newName.trim() && newEmail.trim() && newPassword && !isSubmitting
                                    ? 'bg-primary-600 hover:bg-primary-700 cursor-pointer'
                                    : 'bg-gray-300 dark:bg-gray-700 text-gray-400 cursor-not-allowed'
                            )}
                        >
                            {isSubmitting && <Loader2 className="w-4 h-4 mr-2 animate-spin" />}
                            <span>{t('confirmCreateMember') || '创建并加入企业'}</span>
                        </button>
                    )}
                </div>
            </div>
        </div>
    );
}
