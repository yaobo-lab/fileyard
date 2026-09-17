import { useState, useEffect, useRef } from 'react';
import { Plus, Search, Filter, Mail, CheckCircle, XCircle, Ban, Settings, Building2, ChevronDown, UserPlus } from 'lucide-react';
import clsx from 'clsx';
import { useAuth, useAuthFetch } from '../context/AuthContext';
import { useGlobalSettings } from '../context/GlobalSettingsContext';
import { useTranslations } from '../context/I18nContext';
import { FilterModal } from '../components/FilterModal';
import { InviteUserModal, UserData } from '../components/InviteUserModal';
import { AddMemberModal } from '../components/AddMemberModal';
import { InviteMemberModal } from '../components/InviteMemberModal';
import { UserDetailsModal } from '../components/UserDetailsModal';
import { ManageUserModal } from '../components/ManageUserModal';
import { Avatar } from '../components/Avatar';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import {
    Table,
    TableBody,
    TableCell,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';

interface User {
    id: string;
    name: string;
    email: string;
    role: string;
    status: 'active' | 'inactive';
    avatar_url?: string | null;
    last_active_at?: string | null;
    department_id?: string | null;
    allowed_department_ids?: string[] | null;
    allowed_tenant_ids?: string[] | null;
    tenant_id?: string;
    suspended_at?: string | null;
    suspended_until?: string | null;
    suspension_reason?: string | null;
}

// Helper to check if current user can manage a target user based on role hierarchy
const canManageUser = (currentRole: string, targetRole: string): boolean => {
    const roleHierarchy: Record<string, number> = {
        'SuperAdmin': 4,
        'Admin': 3,
        'Manager': 2,
        'Employee': 1,
    };
    const currentLevel = roleHierarchy[currentRole] || 0;
    const targetLevel = roleHierarchy[targetRole] || 0;
    return currentLevel > targetLevel;
};

// Helper to check if current user can delete (Admin and above only)
const canDeleteUser = (currentRole: string, targetRole: string): boolean => {
    const roleHierarchy: Record<string, number> = {
        'SuperAdmin': 4,
        'Admin': 3,
        'Manager': 2,
        'Employee': 1,
    };
    const currentLevel = roleHierarchy[currentRole] || 0;
    const targetLevel = roleHierarchy[targetRole] || 0;
    return currentLevel >= 3 && currentLevel > targetLevel;
};

export function Users() {
    const t = useTranslations('Users');
    const [users, setUsers] = useState<User[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [searchTerm, setSearchTerm] = useState('');
    const [isFilterOpen, setIsFilterOpen] = useState(false);
    const [isInviteModalOpen, setIsInviteModalOpen] = useState(false);
    const [isAddMemberModalOpen, setIsAddMemberModalOpen] = useState(false);
    const [isInviteMemberModalOpen, setIsInviteMemberModalOpen] = useState(false);
    const [isDetailsModalOpen, setIsDetailsModalOpen] = useState(false);
    const [isManageModalOpen, setIsManageModalOpen] = useState(false);
    const [selectedUser, setSelectedUser] = useState<User | null>(null);
    const [viewingUser, setViewingUser] = useState<User | null>(null);
    const [managingUser, setManagingUser] = useState<User | null>(null);
    const [filters, setFilters] = useState<any>({});
    const [departments, setDepartments] = useState<any[]>([]);
    const [accessibleDepartments, setAccessibleDepartments] = useState<any[]>([]);
    const [selectedDepartment, setSelectedDepartment] = useState<string | null>(null);
    const [isDeptDropdownOpen, setIsDeptDropdownOpen] = useState(false);
    const deptDropdownRef = useRef<HTMLDivElement>(null);
    const { tenant, user: currentUser } = useAuth();
    const authFetch = useAuthFetch();
    const { formatDate, formatDateTime } = useGlobalSettings();

    const getRoleLabel = (role: string) => {
        switch (role) {
            case 'SuperAdmin': return t('roleSuperAdmin');
            case 'Admin': return t('roleAdmin');
            case 'Manager': return t('roleManager');
            case 'Employee': return t('roleEmployee');
            default: return role;
        }
    };

    const getStatusLabel = (status: string) => {
        switch (status) {
            case 'active': return t('statusActive');
            case 'inactive': return t('statusInactive');
            case 'suspended': return t('statusSuspended');
            default: return status;
        }
    };

    const roleFilterOptions = [
        { label: t('roleSuperAdmin'), value: 'SuperAdmin' },
        { label: t('roleAdmin'), value: 'Admin' },
        { label: t('roleManager'), value: 'Manager' },
        { label: t('roleEmployee'), value: 'Employee' },
    ];

    const statusFilterOptions = [
        { label: t('statusActive'), value: 'active' },
        { label: t('statusInactive'), value: 'inactive' },
        { label: t('statusSuspended'), value: 'suspended' },
    ];
    
    const isManager = currentUser?.role === 'Manager';
    const isAdminOrAbove = currentUser?.role === 'SuperAdmin' || currentUser?.role === 'Admin';

    // Close department dropdown when clicking outside
    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (deptDropdownRef.current && !deptDropdownRef.current.contains(event.target as Node)) {
                setIsDeptDropdownOpen(false);
            }
        };
        document.addEventListener('click', handleClickOutside);
        return () => document.removeEventListener('click', handleClickOutside);
    }, []);

    useEffect(() => {
        fetchDepartments();
    }, []);

    const fetchDepartments = async () => {
        try {
            const response = await authFetch('/api/departments');
            if (response.ok) {
                const data = await response.json();
                setDepartments(data);
                
                // For managers, filter to only their accessible departments
                if (isManager && currentUser) {
                    const managerDepts: string[] = [];
                    if (currentUser.department_id) {
                        managerDepts.push(currentUser.department_id);
                    }
                    if (currentUser.allowed_department_ids) {
                        for (const d of currentUser.allowed_department_ids) {
                            if (!managerDepts.includes(d)) {
                                managerDepts.push(d);
                            }
                        }
                    }
                    const filtered = data.filter((d: any) => managerDepts.includes(d.id));
                    setAccessibleDepartments(filtered);
                    // Default to first accessible department
                    if (filtered.length > 0 && !selectedDepartment) {
                        setSelectedDepartment(filtered[0].id);
                    }
                } else {
                    // Admins see all departments
                    setAccessibleDepartments(data);
                }
            }
        } catch (error) {
            console.error('Failed to fetch departments', error);
        }
    };

    const getDepartmentName = (id?: string | null) => {
        if (!id) return '-';
        const dept = departments.find(d => d.id === id);
        return dept ? dept.name : '-';
    };

    useEffect(() => {
        if (tenant) {
            fetchUsers();
        }
    }, [filters, tenant, selectedDepartment]);

    const fetchUsers = async () => {
        try {
            setIsLoading(true);

            const params = new URLSearchParams();
            if (tenant?.id) params.append('tenant_id', tenant.id);
            if (filters.role) params.append('role', filters.role);
            if (filters.status) params.append('status', filters.status);
            if (filters.search) params.append('search', filters.search);
            // Add department filter for managers
            if (selectedDepartment) {
                params.append('department_id', selectedDepartment);
            }

            const response = await authFetch(`/api/users?${params.toString()}`);

            if (!response.ok) throw new Error('Failed to fetch users');
            const data = await response.json();
            setUsers(data);
        } catch (error) {
            console.error('Error fetching users:', error);
        } finally {
            setIsLoading(false);
        }
    };

    const getInitials = (name: string) => {
        return name.split(' ').map(n => n[0]).join('').toUpperCase();
    };

    const handleInvite = async (data: UserData) => {
        const response = await authFetch('/api/users', {
            method: 'POST',
            body: JSON.stringify(data),
        });

        if (!response.ok) {
            throw new Error('Failed to invite user');
        }

        fetchUsers();
    };

    const handleEdit = (user: User) => {
        setSelectedUser(user);
        setIsInviteModalOpen(true);
    };

    const handleUpdateUser = async (data: UserData) => {
        if (!selectedUser) return;

        const updatePayload: Record<string, any> = {
            name: data.name,
            role: data.role,
            department_id: data.department_id || null,
            allowed_department_ids: data.allowed_department_ids || [],
        };

        // Only SuperAdmins can modify allowed_tenant_ids
        if (currentUser?.role === 'SuperAdmin' && data.allowed_tenant_ids) {
            updatePayload.allowed_tenant_ids = data.allowed_tenant_ids;
        }

        // Include password confirmation if provided (for role changes)
        if (data.confirm_password) {
            updatePayload.confirm_password = data.confirm_password;
        }

        const response = await authFetch(`/api/users/${selectedUser.id}`, {
            method: 'PUT',
            body: JSON.stringify(updatePayload),
        });

        if (!response.ok) {
            const errorData = await response.json().catch(() => ({}));
            if (response.status === 403) {
                throw new Error('Incorrect password or insufficient permissions');
            }
            throw new Error(errorData.message || 'Failed to update user');
        }

        fetchUsers();
        setSelectedUser(null);
    };

    const handleModalSubmit = async (data: UserData) => {
        if (selectedUser) {
            await handleUpdateUser(data);
        } else {
            await handleInvite(data);
        }
    };

    // Manage User Modal handlers
    const handleSuspend = async (data: { until: string | null; reason: string }) => {
        if (!managingUser) return;

        const response = await authFetch(`/api/users/${managingUser.id}/suspend`, {
            method: 'POST',
            body: JSON.stringify({
                until: data.until,
                reason: data.reason,
            }),
        });

        if (!response.ok) {
            const errorData = await response.json().catch(() => ({}));
            throw new Error(errorData.message || 'Failed to suspend user');
        }

        fetchUsers();
    };

    const handleUnsuspend = async () => {
        if (!managingUser) return;

        const response = await authFetch(`/api/users/${managingUser.id}/unsuspend`, {
            method: 'POST',
        });

        if (!response.ok) {
            throw new Error('Failed to unsuspend user');
        }

        fetchUsers();
    };

    const handlePermanentDelete = async () => {
        if (!managingUser) return;

        const response = await authFetch(`/api/users/${managingUser.id}/permanent`, {
            method: 'DELETE',
        });

        if (!response.ok) {
            const errorData = await response.json().catch(() => ({}));
            throw new Error(errorData.message || 'Failed to delete user');
        }

        fetchUsers();
    };

    const handleResetPassword = async (newPassword: string) => {
        if (!managingUser) return;

        const response = await authFetch(`/api/users/${managingUser.id}/reset-password`, {
            method: 'POST',
            body: JSON.stringify({ new_password: newPassword }),
        });

        if (!response.ok) {
            const errorData = await response.json().catch(() => ({}));
            throw new Error(errorData.message || 'Failed to reset password');
        }
    };

    const handleSendResetEmail = async () => {
        if (!managingUser) return;

        const response = await authFetch(`/api/users/${managingUser.id}/send-reset-email`, {
            method: 'POST',
        });

        if (!response.ok) {
            const errorData = await response.json().catch(() => ({}));
            throw new Error(errorData.message || 'Failed to send reset email');
        }
    };

    // Check if current user can reset password for target user
    const canResetPassword = (currentRole: string, targetRole: string): boolean => {
        if (currentRole === 'SuperAdmin') return true;
        if (currentRole === 'Admin' && (targetRole === 'Manager' || targetRole === 'Employee')) return true;
        if (currentRole === 'Manager' && targetRole === 'Employee') return true;
        return false;
    };

    const handleChangeEmail = async (newEmail: string) => {
        if (!managingUser) return;

        const response = await authFetch(`/api/users/${managingUser.id}/change-email`, {
            method: 'POST',
            body: JSON.stringify({ email: newEmail }),
        });

        if (!response.ok) {
            const errorData = await response.json().catch(() => ({}));
            throw new Error(errorData.message || 'Failed to change email');
        }

        fetchUsers();
    };

    // Check if a user is currently suspended
    const isUserSuspended = (user: User): boolean => {
        if (!user.suspended_at) return false;
        if (!user.suspended_until) return true; // Indefinitely suspended
        return new Date(user.suspended_until) > new Date();
    };

    // Format suspension info for display
    const getSuspensionInfo = (user: User): string => {
        if (!user.suspended_until) return t('suspendedIndefinitely');
        return t('suspendedUntil').replace('{time}', formatDateTime(user.suspended_until));
    };

    const filteredUsers = searchTerm
        ? users.filter(u => u.name.toLowerCase().includes(searchTerm.toLowerCase()) || u.email.toLowerCase().includes(searchTerm.toLowerCase()))
        : users;

    const getSelectedDepartmentName = () => {
        if (!selectedDepartment) return t('allDepartments');
        const dept = accessibleDepartments.find(d => d.id === selectedDepartment);
        return dept ? dept.name : t('allDepartments');
    };

    return (
        <div className="space-y-4 sm:space-y-6">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 sm:gap-4">
                <div>
                    <h1 className="text-xl sm:text-2xl font-bold text-foreground">{t('title')}</h1>
                    <p className="text-xs sm:text-sm text-muted-foreground mt-0.5 sm:mt-1">{t('description')}</p>
                </div>
                <div className="flex items-center space-x-2 sm:space-x-3">
                    {/* Department Switcher - always show for Managers and Admins */}
                    {(isManager || isAdminOrAbove) ? (
                        <div className="relative" ref={deptDropdownRef}>
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={(e) => { e.stopPropagation(); setIsDeptDropdownOpen(!isDeptDropdownOpen); }}
                                className="h-9 gap-1.5"
                            >
                                <Building2 className="w-4 h-4 text-muted-foreground" />
                                <span className="hidden sm:inline">{getSelectedDepartmentName()}</span>
                                <ChevronDown className="w-3.5 h-3.5 text-muted-foreground/60" />
                            </Button>
                            {isDeptDropdownOpen && (
                                <div className="absolute right-0 mt-2 w-56 bg-popover rounded-xl shadow-lg border border-border z-50 py-1">
                                    {isAdminOrAbove && (
                                        <button
                                            onClick={() => { setSelectedDepartment(null); setIsDeptDropdownOpen(false); }}
                                            className={clsx(
                                                "w-full text-left px-3 py-1.5 text-sm rounded-lg mx-1 w-[calc(100%-0.5rem)] transition-colors",
                                                !selectedDepartment ? "bg-muted font-medium text-foreground" : "text-foreground hover:bg-muted"
                                            )}
                                        >
                                            {t('allDepartments')}
                                        </button>
                                    )}
                                    {accessibleDepartments.map((dept) => (
                                        <button
                                            key={dept.id}
                                            onClick={() => { setSelectedDepartment(dept.id); setIsDeptDropdownOpen(false); }}
                                            className={clsx(
                                                "w-full text-left px-3 py-1.5 text-sm rounded-lg mx-1 w-[calc(100%-0.5rem)] transition-colors",
                                                selectedDepartment === dept.id ? "bg-muted font-medium text-foreground" : "text-foreground hover:bg-muted"
                                            )}
                                        >
                                            {dept.name}
                                        </button>
                                    ))}
                                </div>
                            )}
                        </div>
                    ) : null}
                    
                    <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setIsInviteMemberModalOpen(true)}
                        className="h-9 gap-1.5 cursor-pointer"
                    >
                        <Mail className="w-4 h-4 text-muted-foreground" />
                        <span className="hidden sm:inline">{t('inviteUser') || '邀请用户'}</span>
                    </Button>
                    <Button
                        size="sm"
                        onClick={() => setIsAddMemberModalOpen(true)}
                        className="h-9 gap-1.5 cursor-pointer bg-primary text-primary-foreground hover:bg-primary/90"
                    >
                        <UserPlus className="w-4 h-4" />
                        <span>{t('addUser') || '添加用户'}</span>
                    </Button>
                </div>
            </div>

            <div className="bg-card border border-border rounded-xl overflow-hidden shadow-xs">
                <div className="p-3 sm:p-4 border-b border-border flex items-center justify-between gap-3 bg-muted/20">
                    <div className="relative max-w-md w-full">
                        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                            <Search className="h-4 w-4 text-muted-foreground/60" />
                        </div>
                        <Input
                            type="text"
                            className="pl-9 h-9 text-sm"
                            placeholder={t('searchPlaceholder')}
                            value={searchTerm}
                            onChange={(e) => setSearchTerm(e.target.value)}
                        />
                    </div>
                    <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setIsFilterOpen(true)}
                        className="h-9 gap-1.5 shrink-0"
                    >
                        <Filter className="w-4 h-4 text-muted-foreground" />
                        <span>{t('filters')}</span>
                        {(filters.role || filters.status) && <span className="ml-1 w-2 h-2 bg-primary rounded-full"></span>}
                    </Button>
                </div>

                <div className="overflow-x-auto">
                    {isLoading ? (
                        <div className="p-8 text-center text-muted-foreground text-sm">{t('loading')}</div>
                    ) : filteredUsers.length === 0 ? (
                        <div className="p-8 text-center text-muted-foreground text-sm">{t('noUsers')}</div>
                    ) : (
                        <>
                            {/* Mobile: Card view */}
                            <div className="sm:hidden divide-y divide-border">
                                {filteredUsers.map((user) => (
                                    <div 
                                        key={user.id} 
                                        className="p-4 hover:bg-muted/30 transition-colors"
                                        onClick={() => {
                                            setViewingUser(user);
                                            setIsDetailsModalOpen(true);
                                        }}
                                    >
                                        <div className="flex items-start justify-between">
                                            <div className="flex items-center min-w-0 flex-1">
                                                <Avatar 
                                                    src={user.avatar_url} 
                                                    name={user.name} 
                                                    size="lg"
                                                />
                                                <div className="ml-3 min-w-0 flex-1">
                                                    <div className="flex items-center gap-1.5">
                                                        <p className="text-sm font-medium text-foreground truncate">
                                                            {user.name}
                                                        </p>
                                                        {tenant?.id && user.tenant_id && user.tenant_id !== tenant.id && (
                                                            <span className="px-1.5 py-0.5 text-[10px] font-normal rounded bg-indigo-50 dark:bg-indigo-900/30 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800 shrink-0">
                                                                {t('crossCompany') || '跨企业'}
                                                            </span>
                                                        )}
                                                    </div>
                                                    <p className="text-xs text-muted-foreground truncate">
                                                        {user.email}
                                                    </p>
                                                    <div className="flex items-center gap-2 mt-1.5 flex-wrap">
                                                        <Badge variant="secondary">
                                                            {getRoleLabel(user.role)}
                                                        </Badge>
                                                        {isUserSuspended(user) ? (
                                                            <Badge variant="destructive" className="gap-1">
                                                                <Ban className="w-3 h-3" />
                                                                {t('statusSuspended')}
                                                            </Badge>
                                                        ) : (
                                                            <Badge
                                                                variant={user.status === 'active' ? 'outline' : 'secondary'}
                                                                className={clsx(
                                                                    "gap-1",
                                                                    user.status === 'active' && "text-emerald-600 dark:text-emerald-400 border-emerald-500/20"
                                                                )}
                                                            >
                                                                {user.status === 'active' ? <CheckCircle className="w-3 h-3" /> : <XCircle className="w-3 h-3" />}
                                                                {getStatusLabel(user.status)}
                                                            </Badge>
                                                        )}
                                                    </div>
                                                </div>
                                            </div>
                                            {/* Action buttons */}
                                            <div className="flex items-center gap-2 ml-2" onClick={e => e.stopPropagation()}>
                                                {currentUser && canManageUser(currentUser.role, user.role) && (
                                                    <Button
                                                        variant="ghost"
                                                        size="sm"
                                                        onClick={() => handleEdit(user)}
                                                        className="h-8 px-2 text-xs"
                                                    >
                                                        {t('edit')}
                                                    </Button>
                                                )}
                                                {currentUser && currentUser.id !== user.id && canManageUser(currentUser.role, user.role) && (
                                                    <Button
                                                        variant="ghost"
                                                        size="icon"
                                                        onClick={() => {
                                                            setManagingUser(user);
                                                            setIsManageModalOpen(true);
                                                        }}
                                                        className="h-8 w-8 text-muted-foreground"
                                                        title={t('manage')}
                                                    >
                                                        <Settings className="w-4 h-4" />
                                                    </Button>
                                                )}
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                            
                            {/* Desktop: Table view */}
                            <Table className="hidden sm:table">
                                <TableHeader>
                                    <TableRow>
                                        <TableHead>{t('colUser')}</TableHead>
                                        <TableHead>{t('colRole')}</TableHead>
                                        <TableHead>{t('colDepartment')}</TableHead>
                                        <TableHead>{t('colStatus')}</TableHead>
                                        <TableHead>{t('colLastActive')}</TableHead>
                                        <TableHead className="text-right">{t('colActions')}</TableHead>
                                    </TableRow>
                                </TableHeader>
                                <TableBody>
                                    {filteredUsers.map((user) => (
                                        <TableRow key={user.id}>
                                            <TableCell className="font-medium">
                                                <div className="flex items-center">
                                                    <Avatar 
                                                        src={user.avatar_url} 
                                                        name={user.name} 
                                                        size="md"
                                                    />
                                                    <div className="ml-3">
                                                        <div className="flex items-center gap-2">
                                                            <button
                                                                onClick={() => {
                                                                    setViewingUser(user);
                                                                    setIsDetailsModalOpen(true);
                                                                }}
                                                                className="text-sm font-semibold text-foreground hover:underline text-left cursor-pointer"
                                                            >
                                                                {user.name}
                                                            </button>
                                                            {tenant?.id && user.tenant_id && user.tenant_id !== tenant.id && (
                                                                <span className="px-1.5 py-0.5 text-[10px] font-normal rounded bg-indigo-50 dark:bg-indigo-900/30 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800 shrink-0">
                                                                    {t('crossCompany') || '跨企业'}
                                                                </span>
                                                            )}
                                                        </div>
                                                        <div className="text-xs text-muted-foreground flex items-center mt-0.5">
                                                            <Mail className="w-3 h-3 mr-1" />
                                                            {user.email}
                                                        </div>
                                                    </div>
                                                </div>
                                            </TableCell>
                                            <TableCell>
                                                <Badge variant="secondary">
                                                    {getRoleLabel(user.role)}
                                                </Badge>
                                            </TableCell>
                                            <TableCell className="text-muted-foreground">
                                                {getDepartmentName(user.department_id)}
                                            </TableCell>
                                            <TableCell>
                                                {isUserSuspended(user) ? (
                                                    <div>
                                                        <Badge variant="destructive" className="gap-1">
                                                            <Ban className="w-3 h-3" />
                                                            {t('statusSuspended')}
                                                        </Badge>
                                                        <p className="text-xs text-muted-foreground mt-0.5">
                                                            {getSuspensionInfo(user)}
                                                        </p>
                                                    </div>
                                                ) : (
                                                    <Badge
                                                        variant={user.status === 'active' ? 'outline' : 'secondary'}
                                                        className={clsx(
                                                            "gap-1",
                                                             user.status === 'active' && "text-emerald-600 dark:text-emerald-400 border-emerald-500/20"
                                                        )}
                                                    >
                                                        {user.status === 'active' ? <CheckCircle className="w-3 h-3" /> : <XCircle className="w-3 h-3" />}
                                                        {getStatusLabel(user.status)}
                                                    </Badge>
                                                )}
                                            </TableCell>
                                            <TableCell className="text-muted-foreground">
                                                {user.last_active_at ? formatDate(user.last_active_at) : '-'}
                                            </TableCell>
                                            <TableCell className="text-right">
                                                <div className="flex items-center justify-end gap-1">
                                                    {currentUser && canManageUser(currentUser.role, user.role) && (
                                                        <Button
                                                            variant="ghost"
                                                            size="sm"
                                                            onClick={() => handleEdit(user)}
                                                            className="h-8 px-2.5 text-xs"
                                                        >
                                                            {t('edit')}
                                                        </Button>
                                                    )}
                                                    {currentUser && currentUser.id !== user.id && canManageUser(currentUser.role, user.role) && (
                                                        <Button
                                                            variant="ghost"
                                                            size="icon"
                                                            onClick={() => {
                                                                setManagingUser(user);
                                                                setIsManageModalOpen(true);
                                                            }}
                                                            className="h-8 w-8 text-muted-foreground"
                                                            title={t('manage')}
                                                        >
                                                            <Settings className="w-4 h-4" />
                                                        </Button>
                                                    )}
                                                </div>
                                            </TableCell>
                                        </TableRow>
                                    ))}
                                </TableBody>
                            </Table>
                        </>
                    )}
                </div>
            </div>

            <FilterModal
                isOpen={isFilterOpen}
                onClose={() => setIsFilterOpen(false)}
                onApply={setFilters}
                config={{
                    role: roleFilterOptions,
                    status: statusFilterOptions,
                    department: departments.map(d => ({ label: d.name, value: d.id })),
                    search: true,
                }}
                initialValues={filters}
            />

            <InviteUserModal
                isOpen={isInviteModalOpen}
                onClose={() => {
                    setIsInviteModalOpen(false);
                    setSelectedUser(null);
                }}
                onSubmit={handleModalSubmit}
                initialData={selectedUser ? {
                    name: selectedUser.name,
                    email: selectedUser.email,
                    role: selectedUser.role,
                    department_id: selectedUser.department_id || '',
                    allowed_department_ids: selectedUser.allowed_department_ids || [],
                    allowed_tenant_ids: selectedUser.allowed_tenant_ids || [],
                    password: '', // Password not editable here
                } : undefined}
            />

            {tenant && (
                <>
                    <AddMemberModal
                        isOpen={isAddMemberModalOpen}
                        onClose={() => setIsAddMemberModalOpen(false)}
                        companyId={tenant.id}
                        companyName={tenant.name}
                        existingUserIds={users.map(u => u.id)}
                        departments={departments}
                        onSuccess={fetchUsers}
                    />

                    <InviteMemberModal
                        isOpen={isInviteMemberModalOpen}
                        onClose={() => setIsInviteMemberModalOpen(false)}
                        companyId={tenant.id}
                        companyName={tenant.name}
                        departments={departments}
                    />
                </>
            )}

            <UserDetailsModal
                isOpen={isDetailsModalOpen}
                onClose={() => {
                    setIsDetailsModalOpen(false);
                    setViewingUser(null);
                }}
                user={viewingUser}
            />

            <ManageUserModal
                isOpen={isManageModalOpen}
                onClose={() => {
                    setIsManageModalOpen(false);
                    setManagingUser(null);
                }}
                user={managingUser}
                onSuspend={handleSuspend}
                onUnsuspend={handleUnsuspend}
                onPermanentDelete={handlePermanentDelete}
                onResetPassword={handleResetPassword}
                onSendResetEmail={handleSendResetEmail}
                onChangeEmail={handleChangeEmail}
                canSuspend={currentUser && managingUser ? canManageUser(currentUser.role, managingUser.role) : false}
                canDelete={currentUser && managingUser ? canDeleteUser(currentUser.role, managingUser.role) : false}
                canResetPassword={currentUser && managingUser ? canResetPassword(currentUser.role, managingUser.role) : false}
            />
        </div>
    );
}
