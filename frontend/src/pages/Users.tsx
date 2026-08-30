import { useState, useEffect, useRef } from 'react';
import { Plus, Search, Filter, Mail, CheckCircle, XCircle, Ban, Settings, Building2, ChevronDown } from 'lucide-react';
import clsx from 'clsx';
import { useAuth, useAuthFetch } from '../context/AuthContext';
import { useGlobalSettings } from '../context/GlobalSettingsContext';
import { FilterModal } from '../components/FilterModal';
import { InviteUserModal, UserData } from '../components/InviteUserModal';
import { UserDetailsModal } from '../components/UserDetailsModal';
import { ManageUserModal } from '../components/ManageUserModal';
import { Avatar } from '../components/Avatar';

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
    suspended_at?: string | null;
    suspended_until?: string | null;
    suspension_reason?: string | null;
}

const roleFilterOptions = [
    { label: 'Super Admin', value: 'SuperAdmin' },
    { label: 'Admin', value: 'Admin' },
    { label: 'Manager', value: 'Manager' },
    { label: 'Employee', value: 'Employee' },
];

const statusFilterOptions = [
    { label: 'Active', value: 'active' },
    { label: 'Inactive', value: 'inactive' },
    { label: 'Suspended', value: 'suspended' },
];

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
    const [users, setUsers] = useState<User[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [searchTerm, setSearchTerm] = useState('');
    const [isFilterOpen, setIsFilterOpen] = useState(false);
    const [isInviteModalOpen, setIsInviteModalOpen] = useState(false);
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
        if (!user.suspended_until) return 'Indefinitely';
        return `Until ${formatDateTime(user.suspended_until)}`;
    };

    const filteredUsers = searchTerm
        ? users.filter(u => u.name.toLowerCase().includes(searchTerm.toLowerCase()) || u.email.toLowerCase().includes(searchTerm.toLowerCase()))
        : users;

    const getSelectedDepartmentName = () => {
        if (!selectedDepartment) return 'All Departments';
        const dept = accessibleDepartments.find(d => d.id === selectedDepartment);
        return dept ? dept.name : 'All Departments';
    };

    return (
        <div className="space-y-4 sm:space-y-6">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 sm:gap-4">
                <div>
                    <h1 className="text-xl sm:text-2xl font-bold text-gray-900 dark:text-white">Users</h1>
                    <p className="text-xs sm:text-sm text-gray-500 dark:text-gray-400 mt-0.5 sm:mt-1">Manage user access and permissions.</p>
                </div>
                <div className="flex items-center space-x-2 sm:space-x-3">
                    {/* Department Switcher - always show for Managers and Admins */}
                    {(isManager || isAdminOrAbove) ? (
                        <div className="relative" ref={deptDropdownRef}>
                            <button
                                onClick={(e) => { e.stopPropagation(); setIsDeptDropdownOpen(!isDeptDropdownOpen); }}
                                className="flex items-center px-3 sm:px-4 py-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg text-sm font-medium text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-700 shadow-sm"
                            >
                                <Building2 className="w-4 h-4 sm:mr-2 text-blue-500" />
                                <span className="hidden sm:inline">{getSelectedDepartmentName()}</span>
                                <ChevronDown className="w-4 h-4 ml-1 sm:ml-2 text-gray-400" />
                            </button>
                            {isDeptDropdownOpen && (
                                <div className="absolute right-0 mt-2 w-56 bg-white dark:bg-gray-800 rounded-lg shadow-lg border border-gray-200 dark:border-gray-700 z-50 py-1">
                                    {isAdminOrAbove && (
                                        <button
                                            onClick={() => { setSelectedDepartment(null); setIsDeptDropdownOpen(false); }}
                                            className={clsx(
                                                "w-full text-left px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700",
                                                !selectedDepartment ? "bg-primary-50 dark:bg-primary-900/20 text-primary-600 dark:text-primary-400" : "text-gray-700 dark:text-gray-200"
                                            )}
                                        >
                                            All Departments
                                        </button>
                                    )}
                                    {accessibleDepartments.map((dept) => (
                                        <button
                                            key={dept.id}
                                            onClick={() => { setSelectedDepartment(dept.id); setIsDeptDropdownOpen(false); }}
                                            className={clsx(
                                                "w-full text-left px-4 py-2 text-sm hover:bg-gray-100 dark:hover:bg-gray-700",
                                                selectedDepartment === dept.id ? "bg-primary-50 dark:bg-primary-900/20 text-primary-600 dark:text-primary-400" : "text-gray-700 dark:text-gray-200"
                                            )}
                                        >
                                            {dept.name}
                                        </button>
                                    ))}
                                </div>
                            )}
                        </div>
                    ) : null}
                    
                    <button
                        onClick={() => {
                            setSelectedUser(null);
                            setIsInviteModalOpen(true);
                        }}
                        className="px-3 sm:px-4 py-2 bg-primary-600 text-white rounded-lg text-sm font-medium hover:bg-primary-700 shadow-sm flex items-center transition-colors">
                        <Plus className="w-4 h-4 sm:mr-2" />
                        <span className="hidden sm:inline">Invite User</span>
                    </button>
                </div>
            </div>

            <div className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-sm overflow-hidden transition-colors">
                <div className="p-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between bg-gray-50 dark:bg-gray-900/50">
                    <div className="relative max-w-md w-full">
                        <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                            <Search className="h-4 w-4 text-gray-400 dark:text-gray-500" />
                        </div>
                        <input
                            type="text"
                            className="block w-full pl-10 pr-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md leading-5 bg-white dark:bg-gray-700 placeholder-gray-500 dark:placeholder-gray-400 text-gray-900 dark:text-white focus:outline-none focus:ring-1 focus:ring-primary-500 focus:border-primary-500 sm:text-sm transition-colors"
                            placeholder="Search users..."
                            value={searchTerm}
                            onChange={(e) => setSearchTerm(e.target.value)}
                        />
                    </div>
                    <button
                        onClick={() => setIsFilterOpen(true)}
                        className="px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md text-sm font-medium text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-600 flex items-center transition-colors">
                        <Filter className="w-4 h-4 mr-2 text-gray-500 dark:text-gray-400" />
                        Filters
                        {(filters.role || filters.status) && <span className="ml-2 w-2 h-2 bg-primary-500 rounded-full"></span>}
                    </button>
                </div>

                <div className="overflow-x-auto">
                    {isLoading ? (
                        <div className="p-8 text-center text-gray-500 dark:text-gray-400">Loading...</div>
                    ) : filteredUsers.length === 0 ? (
                        <div className="p-8 text-center text-gray-500 dark:text-gray-400">No users found</div>
                    ) : (
                        <>
                            {/* Mobile: Card view */}
                            <div className="sm:hidden divide-y divide-gray-200 dark:divide-gray-700">
                                {filteredUsers.map((user) => (
                                    <div 
                                        key={user.id} 
                                        className="p-4 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
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
                                                    <p className="text-sm font-medium text-gray-900 dark:text-white truncate">
                                                        {user.name}
                                                    </p>
                                                    <p className="text-xs text-gray-500 dark:text-gray-400 truncate">
                                                        {user.email}
                                                    </p>
                                                    <div className="flex items-center gap-2 mt-1.5 flex-wrap">
                                                        <span className="px-2 py-0.5 text-xs font-semibold rounded-full bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-300">
                                                            {user.role}
                                                        </span>
                                                        {isUserSuspended(user) ? (
                                                            <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-amber-100 dark:bg-amber-900/30 text-amber-800 dark:text-amber-300">
                                                                <Ban className="w-3 h-3 mr-1" />
                                                                Suspended
                                                            </span>
                                                        ) : (
                                                            <span className={clsx(
                                                                "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium",
                                                                user.status === 'active'
                                                                    ? "bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300"
                                                                    : "bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-300"
                                                            )}>
                                                                {user.status === 'active' ? <CheckCircle className="w-3 h-3 mr-1" /> : <XCircle className="w-3 h-3 mr-1" />}
                                                                {user.status.charAt(0).toUpperCase() + user.status.slice(1)}
                                                            </span>
                                                        )}
                                                    </div>
                                                </div>
                                            </div>
                                            {/* Action buttons */}
                                            <div className="flex items-center gap-2 ml-2" onClick={e => e.stopPropagation()}>
                                                {currentUser && canManageUser(currentUser.role, user.role) && (
                                                    <button
                                                        onClick={() => handleEdit(user)}
                                                        className="p-2 text-primary-600 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-lg"
                                                    >
                                                        Edit
                                                    </button>
                                                )}
                                                {currentUser && currentUser.id !== user.id && canManageUser(currentUser.role, user.role) && (
                                                    <button
                                                        onClick={() => {
                                                            setManagingUser(user);
                                                            setIsManageModalOpen(true);
                                                        }}
                                                        className="p-2 text-gray-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg"
                                                    >
                                                        <Settings className="w-5 h-5" />
                                                    </button>
                                                )}
                                            </div>
                                        </div>
                                    </div>
                                ))}
                            </div>
                            
                            {/* Desktop: Table view */}
                            <table className="hidden sm:table min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                <thead className="bg-gray-50 dark:bg-gray-700/50">
                                    <tr>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">User</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Role</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Department</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Status</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">Last Active</th>
                                        <th className="relative px-6 py-3"><span className="sr-only">Actions</span></th>
                                    </tr>
                                </thead>
                                <tbody className="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                                    {filteredUsers.map((user) => (
                                        <tr key={user.id} className="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                                            <td className="px-6 py-4 whitespace-nowrap">
                                                <div className="flex items-center">
                                                    <Avatar 
                                                        src={user.avatar_url} 
                                                        name={user.name} 
                                                        size="md"
                                                    />
                                                    <div className="ml-4">
                                                        <button
                                                            onClick={() => {
                                                                setViewingUser(user);
                                                                setIsDetailsModalOpen(true);
                                                            }}
                                                            className="text-sm font-medium text-gray-900 dark:text-white hover:text-primary-600 dark:hover:text-primary-400 hover:underline text-left"
                                                        >
                                                            {user.name}
                                                        </button>
                                                        <div className="text-sm text-gray-500 dark:text-gray-400 flex items-center">
                                                            <Mail className="w-3 h-3 mr-1" />
                                                            {user.email}
                                                        </div>
                                                    </div>
                                                </div>
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap">
                                                <span className="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-300">
                                                    {user.role}
                                                </span>
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                {getDepartmentName(user.department_id)}
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap">
                                                {isUserSuspended(user) ? (
                                                    <div>
                                                        <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-amber-100 dark:bg-amber-900/30 text-amber-800 dark:text-amber-300">
                                                            <Ban className="w-3 h-3 mr-1" />
                                                            Suspended
                                                        </span>
                                                        <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                                                            {getSuspensionInfo(user)}
                                                        </p>
                                                    </div>
                                                ) : (
                                                    <span className={clsx(
                                                        "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium",
                                                        user.status === 'active'
                                                            ? "bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300"
                                                            : "bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-300"
                                                    )}>
                                                        {user.status === 'active' ? <CheckCircle className="w-3 h-3 mr-1" /> : <XCircle className="w-3 h-3 mr-1" />}
                                                        {user.status.charAt(0).toUpperCase() + user.status.slice(1)}
                                                    </span>
                                                )}
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                {user.last_active_at ? formatDate(user.last_active_at) : 'Never'}
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium space-x-2">
                                                {/* Edit button - only show if user can manage target */}
                                                {currentUser && canManageUser(currentUser.role, user.role) && (
                                                    <button
                                                        onClick={() => handleEdit(user)}
                                                        className="text-primary-600 hover:text-primary-900 dark:text-primary-400 dark:hover:text-primary-300"
                                                    >
                                                        Edit
                                                    </button>
                                                )}
                                                {/* Manage button - suspend/delete options */}
                                                {currentUser && currentUser.id !== user.id && canManageUser(currentUser.role, user.role) && (
                                                    <button
                                                        onClick={() => {
                                                            setManagingUser(user);
                                                            setIsManageModalOpen(true);
                                                        }}
                                                        className="text-gray-600 hover:text-gray-900 dark:text-gray-400 dark:hover:text-gray-300"
                                                        title="Manage User"
                                                    >
                                                        <Settings className="w-4 h-4 inline" />
                                                    </button>
                                                )}
                                            </td>
                                        </tr>
                                    ))}
                                </tbody>
                            </table>
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
