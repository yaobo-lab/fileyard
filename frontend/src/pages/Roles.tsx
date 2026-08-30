import { useState, useEffect } from 'react';
import {
    Shield,
    Users,
    Briefcase,
    User,
    Plus,
    Settings,
    Trash2,
    ChevronRight,
    ChevronDown,
    Lock,
    Unlock,
    Globe,
    Building2,
    Check,
    X
} from 'lucide-react';
import { useAuth, useAuthFetch } from '../context/AuthContext';
import { useGlobalSettings } from '../context/GlobalSettingsContext';
import { Navigate } from 'react-router-dom';
import clsx from 'clsx';
import { RolePermissionsModal } from '../components/RolePermissionsModal';

interface Role {
    id: string;
    tenant_id: string | null;
    name: string;
    description: string | null;
    base_role: string;
    is_system: boolean;
    created_at: string;
    updated_at: string;
}

interface CreateRoleData {
    name: string;
    description: string;
    base_role: string;
}

export function RolesPage() {
    const { user, tenant } = useAuth();
    const authFetch = useAuthFetch();

    // Admin and SuperAdmin can access Roles page
    if (!user || !['SuperAdmin', 'Admin'].includes(user.role)) {
        return <Navigate to="/" replace />;
    }

    const [roles, setRoles] = useState<Role[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const { formatDate } = useGlobalSettings();
    const [showCreateModal, setShowCreateModal] = useState(false);
    const [selectedRole, setSelectedRole] = useState<Role | null>(null);
    const [showPermissionsModal, setShowPermissionsModal] = useState(false);

    // Create form state
    const [newRoleName, setNewRoleName] = useState('');
    const [newRoleDescription, setNewRoleDescription] = useState('');
    const [newRoleBaseRole, setNewRoleBaseRole] = useState('Employee');
    const [isGlobalRole, setIsGlobalRole] = useState(false);
    const [isCreating, setIsCreating] = useState(false);

    const canManageRoles = ['Admin', 'SuperAdmin'].includes(user?.role || '');
    const isSuperAdmin = user?.role === 'SuperAdmin';

    useEffect(() => {
        fetchRoles();
    }, []);

    const fetchRoles = async () => {
        setIsLoading(true);
        try {
            const response = await authFetch('/api/roles?include_global=true');
            if (response.ok) {
                const data = await response.json();
                setRoles(data);
            }
        } catch (error) {
            console.error('Failed to fetch roles', error);
        } finally {
            setIsLoading(false);
        }
    };

    const handleCreateRole = async () => {
        if (!newRoleName.trim()) return;

        setIsCreating(true);
        try {
            const url = isGlobalRole && isSuperAdmin 
                ? '/api/roles?is_global=true' 
                : '/api/roles';
            
            const response = await authFetch(url, {
                method: 'POST',
                body: JSON.stringify({
                    name: newRoleName,
                    description: newRoleDescription || null,
                    base_role: newRoleBaseRole,
                }),
            });

            if (response.ok) {
                setShowCreateModal(false);
                setNewRoleName('');
                setNewRoleDescription('');
                setNewRoleBaseRole('Employee');
                setIsGlobalRole(false);
                fetchRoles();
            }
        } catch (error) {
            console.error('Failed to create role', error);
        } finally {
            setIsCreating(false);
        }
    };

    const handleDeleteRole = async (roleId: string) => {
        if (!confirm('Are you sure you want to delete this role? This cannot be undone.')) {
            return;
        }

        try {
            const response = await authFetch(`/api/roles/${roleId}`, {
                method: 'DELETE',
            });

            if (response.ok) {
                fetchRoles();
            } else if (response.status === 409) {
                alert('Cannot delete role: it is currently assigned to users.');
            }
        } catch (error) {
            console.error('Failed to delete role', error);
        }
    };

    const getRoleIcon = (baseRole: string) => {
        switch (baseRole) {
            case 'SuperAdmin':
                return Shield;
            case 'Admin':
                return Users;
            case 'Manager':
                return Briefcase;
            default:
                return User;
        }
    };

    const getRoleColors = (baseRole: string) => {
        switch (baseRole) {
            case 'SuperAdmin':
                return {
                    text: 'text-purple-600 dark:text-purple-400',
                    bg: 'bg-purple-50 dark:bg-purple-900/20',
                    border: 'border-purple-200 dark:border-purple-800',
                };
            case 'Admin':
                return {
                    text: 'text-blue-600 dark:text-blue-400',
                    bg: 'bg-blue-50 dark:bg-blue-900/20',
                    border: 'border-blue-200 dark:border-blue-800',
                };
            case 'Manager':
                return {
                    text: 'text-green-600 dark:text-green-400',
                    bg: 'bg-green-50 dark:bg-green-900/20',
                    border: 'border-green-200 dark:border-green-800',
                };
            default:
                return {
                    text: 'text-gray-600 dark:text-gray-400',
                    bg: 'bg-gray-50 dark:bg-gray-700/50',
                    border: 'border-gray-200 dark:border-gray-700',
                };
        }
    };

    // Separate system roles from custom roles
    const systemRoles = roles.filter(r => r.is_system);
    const customRoles = roles.filter(r => !r.is_system);
    const globalCustomRoles = customRoles.filter(r => r.tenant_id === null);
    const tenantCustomRoles = customRoles.filter(r => r.tenant_id !== null);

    if (isLoading) {
        return (
            <div className="flex items-center justify-center h-96">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
            </div>
        );
    }

    return (
        <div className="p-4 sm:p-6 max-w-6xl mx-auto">
            <div className="mb-6 sm:mb-8 flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 sm:gap-4">
                <div>
                    <h1 className="text-2xl sm:text-3xl font-bold text-gray-900 dark:text-white flex items-center gap-2 sm:gap-3">
                        <Shield className="w-6 h-6 sm:w-8 sm:h-8 text-primary-600" />
                        Roles & Permissions
                    </h1>
                    <p className="mt-1 sm:mt-2 text-xs sm:text-sm text-gray-600 dark:text-gray-400">
                        Manage user roles and their access levels
                    </p>
                </div>
                {canManageRoles && (
                    <button
                        onClick={() => setShowCreateModal(true)}
                        className="flex items-center px-3 sm:px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 text-sm font-medium transition-colors self-start sm:self-auto"
                        title="Create Custom Role"
                    >
                        <Plus className="w-4 h-4 sm:mr-2" />
                        <span className="hidden sm:inline">Create Custom Role</span>
                    </button>
                )}
            </div>

            {/* System Roles */}
            <div className="mb-6 sm:mb-8">
                <div className="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-2 mb-4">
                    <div className="flex items-center gap-2">
                        <Lock className="w-4 h-4 text-gray-400" />
                        <h2 className="text-base sm:text-lg font-semibold text-gray-900 dark:text-white">System Roles</h2>
                    </div>
                    <span className="text-xs text-gray-500 dark:text-gray-400 ml-6 sm:ml-0">
                        {isSuperAdmin ? '(Built-in, editable by SuperAdmin)' : '(Built-in, read-only)'}
                    </span>
                </div>
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                    {systemRoles.map((role) => {
                        const Icon = getRoleIcon(role.base_role);
                        const colors = getRoleColors(role.base_role);

                        return (
                            <div
                                key={role.id}
                                className={clsx(
                                    "bg-white dark:bg-gray-800 border rounded-xl p-5 transition-all",
                                    colors.border
                                )}
                            >
                                <div className="flex items-start justify-between mb-3">
                                    <div className={clsx("p-2.5 rounded-lg", colors.bg)}>
                                        <Icon className={clsx("w-5 h-5", colors.text)} />
                                    </div>
                                    <div className="flex items-center gap-1">
                                        <Globe className="w-3 h-3 text-gray-400" />
                                        <span className="text-xs text-gray-400">Global</span>
                                    </div>
                                </div>
                                <h3 className={clsx("text-lg font-semibold mb-1", colors.text)}>
                                    {role.name}
                                </h3>
                                <p className="text-sm text-gray-500 dark:text-gray-400 mb-3 line-clamp-2">
                                    {role.description || 'No description'}
                                </p>
                                <button
                                    onClick={() => {
                                        setSelectedRole(role);
                                        setShowPermissionsModal(true);
                                    }}
                                    className="text-sm text-primary-600 hover:text-primary-700 font-medium flex items-center"
                                >
                                    {isSuperAdmin ? 'Edit Permissions' : 'View Permissions'}
                                    <ChevronRight className="w-4 h-4 ml-1" />
                                </button>
                            </div>
                        );
                    })}
                </div>
            </div>

            {/* Custom Roles */}
            <div className="mb-6 sm:mb-8">
                <div className="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-2 mb-4">
                    <div className="flex items-center gap-2">
                        <Unlock className="w-4 h-4 text-primary-500" />
                        <h2 className="text-base sm:text-lg font-semibold text-gray-900 dark:text-white">Custom Roles</h2>
                    </div>
                    <span className="text-xs text-gray-500 dark:text-gray-400 ml-6 sm:ml-0">(Created by your organization)</span>
                </div>

                {customRoles.length === 0 ? (
                    <div className="bg-gray-50 dark:bg-gray-800/50 border border-dashed border-gray-300 dark:border-gray-700 rounded-xl p-8 text-center">
                        <Shield className="w-12 h-12 mx-auto mb-3 text-gray-300 dark:text-gray-600" />
                        <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-1">
                            No Custom Roles Yet
                        </h3>
                        <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
                            Create custom roles to define specific permission sets for your team.
                        </p>
                        {canManageRoles && (
                            <button
                                onClick={() => setShowCreateModal(true)}
                                className="inline-flex items-center px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 text-sm font-medium"
                            >
                                <Plus className="w-4 h-4 mr-2" />
                                Create Your First Role
                            </button>
                        )}
                    </div>
                ) : (
                    <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
                        {/* Mobile: Card view */}
                        <div className="sm:hidden divide-y divide-gray-200 dark:divide-gray-700">
                            {customRoles.map((role) => {
                                const Icon = getRoleIcon(role.base_role);
                                const colors = getRoleColors(role.base_role);

                                return (
                                    <div key={role.id} className="p-4">
                                        <div className="flex items-start justify-between gap-3">
                                            <div className="flex items-center gap-3 min-w-0">
                                                <div className={clsx("p-2 rounded-lg flex-shrink-0", colors.bg)}>
                                                    <Icon className={clsx("w-4 h-4", colors.text)} />
                                                </div>
                                                <div className="min-w-0">
                                                    <p className="font-medium text-gray-900 dark:text-white truncate">{role.name}</p>
                                                    <p className="text-xs text-gray-500 dark:text-gray-400 truncate">
                                                        {role.description || 'No description'}
                                                    </p>
                                                </div>
                                            </div>
                                            <div className="flex items-center gap-1 flex-shrink-0">
                                                <button
                                                    onClick={() => {
                                                        setSelectedRole(role);
                                                        setShowPermissionsModal(true);
                                                    }}
                                                    className="p-2 text-gray-400 hover:text-primary-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                                                    title="View/Edit Permissions"
                                                >
                                                    <Settings className="w-4 h-4" />
                                                </button>
                                                {canManageRoles && role.tenant_id && (
                                                    <button
                                                        onClick={() => handleDeleteRole(role.id)}
                                                        className="p-2 text-gray-400 hover:text-red-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                                                        title="Delete Role"
                                                    >
                                                        <Trash2 className="w-4 h-4" />
                                                    </button>
                                                )}
                                            </div>
                                        </div>
                                        <div className="flex items-center gap-3 mt-3 flex-wrap">
                                            <span className={clsx(
                                                "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium",
                                                colors.bg, colors.text
                                            )}>
                                                {role.base_role}
                                            </span>
                                            {role.tenant_id ? (
                                                <span className="inline-flex items-center text-xs text-gray-500 dark:text-gray-400">
                                                    <Building2 className="w-3 h-3 mr-1" />
                                                    {tenant?.name || 'This Company'}
                                                </span>
                                            ) : (
                                                <span className="inline-flex items-center text-xs text-gray-500 dark:text-gray-400">
                                                    <Globe className="w-3 h-3 mr-1" />
                                                    Global
                                                </span>
                                            )}
                                            <span className="text-xs text-gray-400 dark:text-gray-500">
                                                {formatDate(role.created_at)}
                                            </span>
                                        </div>
                                    </div>
                                );
                            })}
                        </div>

                        {/* Desktop: Table view */}
                        <table className="hidden sm:table min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                            <thead className="bg-gray-50 dark:bg-gray-900/50">
                                <tr>
                                    <th className="px-6 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                        Role
                                    </th>
                                    <th className="px-6 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                        Base Level
                                    </th>
                                    <th className="px-6 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                        Scope
                                    </th>
                                    <th className="px-6 py-3 text-left text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                        Created
                                    </th>
                                    <th className="px-6 py-3 text-right text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                                        Actions
                                    </th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                                {customRoles.map((role) => {
                                    const Icon = getRoleIcon(role.base_role);
                                    const colors = getRoleColors(role.base_role);

                                    return (
                                        <tr key={role.id} className="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors">
                                            <td className="px-6 py-4 whitespace-nowrap">
                                                <div className="flex items-center">
                                                    <div className={clsx("p-2 rounded-lg mr-3", colors.bg)}>
                                                        <Icon className={clsx("w-4 h-4", colors.text)} />
                                                    </div>
                                                    <div>
                                                        <div className="text-sm font-medium text-gray-900 dark:text-white">
                                                            {role.name}
                                                        </div>
                                                        <div className="text-xs text-gray-500 dark:text-gray-400 truncate max-w-xs">
                                                            {role.description || 'No description'}
                                                        </div>
                                                    </div>
                                                </div>
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap">
                                                <span className={clsx(
                                                    "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium",
                                                    colors.bg, colors.text
                                                )}>
                                                    {role.base_role}
                                                </span>
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap">
                                                {role.tenant_id ? (
                                                    <span className="inline-flex items-center text-xs text-gray-500 dark:text-gray-400">
                                                        <Building2 className="w-3 h-3 mr-1" />
                                                        {tenant?.name || 'This Company'}
                                                    </span>
                                                ) : (
                                                    <span className="inline-flex items-center text-xs text-gray-500 dark:text-gray-400">
                                                        <Globe className="w-3 h-3 mr-1" />
                                                        Global
                                                    </span>
                                                )}
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                {formatDate(role.created_at)}
                                            </td>
                                            <td className="px-6 py-4 whitespace-nowrap text-right">
                                                <div className="flex items-center justify-end gap-2">
                                                    <button
                                                        onClick={() => {
                                                            setSelectedRole(role);
                                                            setShowPermissionsModal(true);
                                                        }}
                                                        className="p-1.5 text-gray-400 hover:text-primary-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                                                        title="View/Edit Permissions"
                                                    >
                                                        <Settings className="w-4 h-4" />
                                                    </button>
                                                    {canManageRoles && role.tenant_id && (
                                                        <button
                                                            onClick={() => handleDeleteRole(role.id)}
                                                            className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                                                            title="Delete Role"
                                                        >
                                                            <Trash2 className="w-4 h-4" />
                                                        </button>
                                                    )}
                                                </div>
                                            </td>
                                        </tr>
                                    );
                                })}
                            </tbody>
                        </table>
                    </div>
                )}
            </div>

            {/* Info Panel */}
            <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4 sm:p-6">
                <h4 className="font-semibold text-blue-900 dark:text-blue-100 mb-2 text-sm sm:text-base">
                    Understanding Role Hierarchy
                </h4>
                <p className="text-xs sm:text-sm text-blue-800 dark:text-blue-200 mb-3">
                    Roles in ClovaLink follow a hierarchical permission model. Each role inherits all permissions from the level below it:
                </p>
                {/* Mobile: Vertical layout */}
                <div className="flex sm:hidden flex-col items-start gap-1 text-sm text-blue-700 dark:text-blue-300">
                    <span className="px-2 py-0.5 bg-gray-100 dark:bg-gray-700 rounded text-gray-700 dark:text-gray-300">Employee</span>
                    <ChevronDown className="w-4 h-4 ml-2" />
                    <span className="px-2 py-0.5 bg-green-100 dark:bg-green-900/30 rounded text-green-700 dark:text-green-400">Manager</span>
                    <ChevronDown className="w-4 h-4 ml-2" />
                    <span className="px-2 py-0.5 bg-blue-100 dark:bg-blue-900/30 rounded text-blue-700 dark:text-blue-400">Admin</span>
                    <ChevronDown className="w-4 h-4 ml-2" />
                    <span className="px-2 py-0.5 bg-purple-100 dark:bg-purple-900/30 rounded text-purple-700 dark:text-purple-400">SuperAdmin</span>
                </div>
                {/* Desktop: Horizontal layout */}
                <div className="hidden sm:flex items-center gap-2 text-sm text-blue-700 dark:text-blue-300">
                    <span className="px-2 py-0.5 bg-gray-100 dark:bg-gray-700 rounded text-gray-700 dark:text-gray-300">Employee</span>
                    <ChevronRight className="w-4 h-4" />
                    <span className="px-2 py-0.5 bg-green-100 dark:bg-green-900/30 rounded text-green-700 dark:text-green-400">Manager</span>
                    <ChevronRight className="w-4 h-4" />
                    <span className="px-2 py-0.5 bg-blue-100 dark:bg-blue-900/30 rounded text-blue-700 dark:text-blue-400">Admin</span>
                    <ChevronRight className="w-4 h-4" />
                    <span className="px-2 py-0.5 bg-purple-100 dark:bg-purple-900/30 rounded text-purple-700 dark:text-purple-400">SuperAdmin</span>
                </div>
            </div>

            {/* Create Role Modal */}
            {showCreateModal && (
                <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
                    <div className="bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-md w-full">
                        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
                            <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                                Create Custom Role
                            </h3>
                            <button
                                onClick={() => setShowCreateModal(false)}
                                className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                            >
                                <X className="w-5 h-5" />
                            </button>
                        </div>
                        <div className="p-6 space-y-4">
                            <div>
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    Role Name *
                                </label>
                                <input
                                    type="text"
                                    value={newRoleName}
                                    onChange={(e) => setNewRoleName(e.target.value)}
                                    placeholder="e.g., Senior Manager"
                                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                />
                            </div>
                            <div>
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    Description
                                </label>
                                <textarea
                                    value={newRoleDescription}
                                    onChange={(e) => setNewRoleDescription(e.target.value)}
                                    placeholder="What is this role for?"
                                    rows={2}
                                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                />
                            </div>
                            {isSuperAdmin && (
                                <div>
                                    <label className="flex items-center p-3 border border-gray-200 dark:border-gray-700 rounded-lg cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700/50">
                                        <input
                                            type="checkbox"
                                            checked={isGlobalRole}
                                            onChange={(e) => setIsGlobalRole(e.target.checked)}
                                            className="h-4 w-4 text-primary-600 focus:ring-primary-500 border-gray-300 rounded"
                                        />
                                        <div className="ml-3">
                                            <span className="text-sm font-medium text-gray-900 dark:text-white flex items-center gap-2">
                                                <Globe className="w-4 h-4 text-primary-500" />
                                                Create as Global Role
                                            </span>
                                            <p className="text-xs text-gray-500 dark:text-gray-400">
                                                Global roles are available to all companies
                                            </p>
                                        </div>
                                    </label>
                                </div>
                            )}
                            <div>
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    Base Permission Level *
                                </label>
                                <p className="text-xs text-gray-500 dark:text-gray-400 mb-2">
                                    This role will inherit all permissions from the selected level.
                                </p>
                                <div className="space-y-2">
                                    {['Employee', 'Manager', 'Admin'].map((level) => {
                                        const colors = getRoleColors(level);
                                        return (
                                            <label
                                                key={level}
                                                className={clsx(
                                                    "flex items-center p-3 border rounded-lg cursor-pointer transition-colors",
                                                    newRoleBaseRole === level
                                                        ? `${colors.border} ${colors.bg}`
                                                        : "border-gray-200 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-700/50"
                                                )}
                                            >
                                                <input
                                                    type="radio"
                                                    name="baseRole"
                                                    value={level}
                                                    checked={newRoleBaseRole === level}
                                                    onChange={(e) => setNewRoleBaseRole(e.target.value)}
                                                    className="sr-only"
                                                />
                                                <span className={clsx("font-medium text-sm", colors.text)}>
                                                    {level}
                                                </span>
                                                {newRoleBaseRole === level && (
                                                    <Check className={clsx("w-4 h-4 ml-auto", colors.text)} />
                                                )}
                                            </label>
                                        );
                                    })}
                                </div>
                            </div>
                        </div>
                        <div className="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-end gap-3">
                            <button
                                onClick={() => setShowCreateModal(false)}
                                className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                            >
                                Cancel
                            </button>
                            <button
                                onClick={handleCreateRole}
                                disabled={!newRoleName.trim() || isCreating}
                                className="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 text-sm font-medium disabled:opacity-50 transition-colors"
                            >
                                {isCreating ? 'Creating...' : 'Create Role'}
                            </button>
                        </div>
                    </div>
            </div>
            )}

            {/* Permissions Modal */}
            {showPermissionsModal && selectedRole && (
                <RolePermissionsModal
                    role={selectedRole}
                    isOpen={showPermissionsModal}
                    onClose={() => {
                        setShowPermissionsModal(false);
                        setSelectedRole(null);
                    }}
                    onSave={() => {
                        setShowPermissionsModal(false);
                        setSelectedRole(null);
                        fetchRoles();
                    }}
                    canEdit={isSuperAdmin || (canManageRoles && !selectedRole.is_system && selectedRole.tenant_id !== null)}
                />
            )}
        </div>
    );
}
