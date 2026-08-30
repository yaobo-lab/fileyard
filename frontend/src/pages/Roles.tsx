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
                    text: 'text-violet-600 dark:text-violet-400',
                    bg: 'bg-violet-500/10',
                    border: 'border-violet-500/25',
                };
            case 'Admin':
                return {
                    text: 'text-sky-600 dark:text-sky-400',
                    bg: 'bg-sky-500/10',
                    border: 'border-sky-500/25',
                };
            case 'Manager':
                return {
                    text: 'text-emerald-600 dark:text-emerald-400',
                    bg: 'bg-emerald-500/10',
                    border: 'border-emerald-500/25',
                };
            default:
                return {
                    text: 'text-muted-foreground',
                    bg: 'bg-muted',
                    border: 'border-border',
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
                <div className="animate-spin rounded-full h-6 w-6 border-2 border-foreground border-t-transparent"></div>
            </div>
        );
    }

    return (
        <div className="max-w-6xl mx-auto space-y-8">
            <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 sm:gap-4">
                <div>
                    <h1 className="text-xl sm:text-2xl font-bold text-foreground flex items-center gap-2 sm:gap-3">
                        <Shield className="w-5 h-5 sm:w-6 sm:h-6 text-foreground" />
                        Roles & Permissions
                    </h1>
                    <p className="mt-1 text-xs sm:text-sm text-muted-foreground">
                        Manage user roles and their access levels
                    </p>
                </div>
                {canManageRoles && (
                    <button
                        onClick={() => setShowCreateModal(true)}
                        className="flex items-center px-3 sm:px-4 py-1.5 bg-foreground text-background rounded-lg hover:bg-foreground/90 text-sm font-medium transition-colors self-start sm:self-auto"
                        title="Create Custom Role"
                    >
                        <Plus className="w-4 h-4 sm:mr-1.5" />
                        <span className="hidden sm:inline">Create Custom Role</span>
                    </button>
                )}
            </div>

            {/* System Roles */}
            <div>
                <div className="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-2 mb-4">
                    <div className="flex items-center gap-2">
                        <Lock className="w-4 h-4 text-muted-foreground/60" />
                        <h2 className="text-sm font-semibold text-foreground">System Roles</h2>
                    </div>
                    <span className="text-xs text-muted-foreground ml-6 sm:ml-0">
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
                                    "bg-card border rounded-xl p-5 transition-all",
                                    colors.border
                                )}
                            >
                                <div className="flex items-start justify-between mb-3">
                                    <div className={clsx("p-2.5 rounded-lg", colors.bg)}>
                                        <Icon className={clsx("w-5 h-5", colors.text)} />
                                    </div>
                                    <div className="flex items-center gap-1">
                                        <Globe className="w-3.5 h-3.5 text-muted-foreground/60" />
                                        <span className="text-xs text-muted-foreground">Global</span>
                                    </div>
                                </div>
                                <h3 className={clsx("text-base font-semibold mb-1", colors.text)}>
                                    {role.name}
                                </h3>
                                <p className="text-sm text-muted-foreground mb-3 line-clamp-2">
                                    {role.description || 'No description'}
                                </p>
                                <button
                                    onClick={() => {
                                        setSelectedRole(role);
                                        setShowPermissionsModal(true);
                                    }}
                                    className="text-sm text-foreground hover:underline font-semibold flex items-center"
                                >
                                    {isSuperAdmin ? 'Edit Permissions' : 'View Permissions'}
                                    <ChevronRight className="w-4 h-4 ml-0.5" />
                                </button>
                            </div>
                        );
                    })}
                </div>
            </div>

            {/* Custom Roles */}
            <div>
                <div className="flex flex-col sm:flex-row sm:items-center gap-1 sm:gap-2 mb-4">
                    <div className="flex items-center gap-2">
                        <Unlock className="w-4 h-4 text-muted-foreground" />
                        <h2 className="text-sm font-semibold text-foreground">Custom Roles</h2>
                    </div>
                    <span className="text-xs text-muted-foreground ml-6 sm:ml-0">(Created by your organization)</span>
                </div>

                {customRoles.length === 0 ? (
                    <div className="bg-muted/10 border border-dashed border-border rounded-xl p-8 text-center">
                        <Shield className="w-10 h-10 mx-auto mb-3 text-muted-foreground/40" />
                        <h3 className="text-sm font-semibold text-foreground mb-1">
                            No Custom Roles Yet
                        </h3>
                        <p className="text-xs text-muted-foreground mb-4">
                            Create custom roles to define specific permission sets for your team.
                        </p>
                        {canManageRoles && (
                            <button
                                onClick={() => setShowCreateModal(true)}
                                className="inline-flex items-center px-3 py-1.5 bg-foreground text-background rounded-lg hover:bg-foreground/90 text-sm font-medium"
                            >
                                <Plus className="w-4 h-4 mr-1.5" />
                                Create Your First Role
                            </button>
                        )}
                    </div>
                ) : (
                    <div className="bg-card rounded-xl border border-border overflow-hidden">
                        {/* Mobile: Card view */}
                        <div className="sm:hidden divide-y divide-border">
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
                                                    <p className="font-semibold text-foreground truncate">{role.name}</p>
                                                    <p className="text-xs text-muted-foreground truncate">
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
                                                    className="p-1.5 text-muted-foreground hover:text-foreground hover:bg-muted rounded-lg transition-colors"
                                                    title="View/Edit Permissions"
                                                >
                                                    <Settings className="w-4 h-4" />
                                                </button>
                                                {canManageRoles && role.tenant_id && (
                                                    <button
                                                        onClick={() => handleDeleteRole(role.id)}
                                                        className="p-1.5 text-muted-foreground hover:text-destructive hover:bg-muted rounded-lg transition-colors"
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
                                                <span className="inline-flex items-center text-xs text-muted-foreground">
                                                    <Building2 className="w-3 h-3 mr-1" />
                                                    {tenant?.name || 'This Company'}
                                                </span>
                                            ) : (
                                                <span className="inline-flex items-center text-xs text-muted-foreground">
                                                    <Globe className="w-3 h-3 mr-1" />
                                                    Global
                                                </span>
                                            )}
                                            <span className="text-xs text-muted-foreground/60">
                                                {formatDate(role.created_at)}
                                            </span>
                                        </div>
                                    </div>
                                );
                            })}
                        </div>

                        {/* Desktop: Table view */}
                        <table className="hidden sm:table min-w-full divide-y divide-border">
                            <thead className="bg-muted/50">
                                <tr>
                                    <th className="px-5 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                                        Role
                                    </th>
                                    <th className="px-5 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                                        Base Level
                                    </th>
                                    <th className="px-5 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                                        Scope
                                    </th>
                                    <th className="px-5 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                                        Created
                                    </th>
                                    <th className="px-5 py-3 text-right text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                                        Actions
                                    </th>
                                </tr>
                            </thead>
                            <tbody className="divide-y divide-border">
                                {customRoles.map((role) => {
                                    const Icon = getRoleIcon(role.base_role);
                                    const colors = getRoleColors(role.base_role);

                                    return (
                                        <tr key={role.id} className="hover:bg-muted/30 transition-colors">
                                            <td className="px-5 py-3.5 whitespace-nowrap">
                                                <div className="flex items-center">
                                                    <div className={clsx("p-2 rounded-lg mr-3", colors.bg)}>
                                                        <Icon className={clsx("w-4 h-4", colors.text)} />
                                                    </div>
                                                    <div>
                                                        <div className="text-sm font-semibold text-foreground">
                                                            {role.name}
                                                        </div>
                                                        <div className="text-xs text-muted-foreground truncate max-w-xs mt-0.5">
                                                            {role.description || 'No description'}
                                                        </div>
                                                    </div>
                                                </div>
                                            </td>
                                            <td className="px-5 py-3.5 whitespace-nowrap">
                                                <span className={clsx(
                                                    "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium",
                                                    colors.bg, colors.text
                                                )}>
                                                    {role.base_role}
                                                </span>
                                            </td>
                                            <td className="px-5 py-3.5 whitespace-nowrap">
                                                {role.tenant_id ? (
                                                    <span className="inline-flex items-center text-xs text-muted-foreground">
                                                        <Building2 className="w-3.5 h-3.5 mr-1" />
                                                        {tenant?.name || 'This Company'}
                                                    </span>
                                                ) : (
                                                    <span className="inline-flex items-center text-xs text-muted-foreground">
                                                        <Globe className="w-3.5 h-3.5 mr-1" />
                                                        Global
                                                    </span>
                                                )}
                                            </td>
                                            <td className="px-5 py-3.5 whitespace-nowrap text-sm text-muted-foreground">
                                                {formatDate(role.created_at)}
                                            </td>
                                            <td className="px-5 py-3.5 whitespace-nowrap text-right">
                                                <div className="flex items-center justify-end gap-1">
                                                    <button
                                                        onClick={() => {
                                                            setSelectedRole(role);
                                                            setShowPermissionsModal(true);
                                                        }}
                                                        className="p-1.5 text-muted-foreground hover:text-foreground hover:bg-muted rounded-lg transition-colors"
                                                        title="View/Edit Permissions"
                                                    >
                                                        <Settings className="w-4 h-4" />
                                                    </button>
                                                    {canManageRoles && role.tenant_id && (
                                                        <button
                                                            onClick={() => handleDeleteRole(role.id)}
                                                            className="p-1.5 text-muted-foreground hover:text-destructive hover:bg-muted rounded-lg transition-colors"
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
            <div className="bg-muted/50 border border-border rounded-xl p-5 sm:p-6">
                <h4 className="font-semibold text-foreground mb-2 text-sm sm:text-base">
                    Understanding Role Hierarchy
                </h4>
                <p className="text-xs sm:text-sm text-muted-foreground mb-4">
                    Roles follow a hierarchical permission model. Each role inherits all permissions from the level below it:
                </p>
                {/* Mobile: Vertical layout */}
                <div className="flex sm:hidden flex-col items-start gap-1 text-sm">
                    <span className="px-2.5 py-0.5 bg-muted rounded text-muted-foreground font-semibold text-xs">Employee</span>
                    <ChevronDown className="w-4 h-4 ml-2 text-muted-foreground/60" />
                    <span className="px-2.5 py-0.5 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 rounded font-semibold text-xs">Manager</span>
                    <ChevronDown className="w-4 h-4 ml-2 text-muted-foreground/60" />
                    <span className="px-2.5 py-0.5 bg-sky-500/10 text-sky-600 dark:text-sky-400 rounded font-semibold text-xs">Admin</span>
                    <ChevronDown className="w-4 h-4 ml-2 text-muted-foreground/60" />
                    <span className="px-2.5 py-0.5 bg-violet-500/10 text-violet-600 dark:text-violet-400 rounded font-semibold text-xs">SuperAdmin</span>
                </div>
                {/* Desktop: Horizontal layout */}
                <div className="hidden sm:flex items-center gap-2 text-sm">
                    <span className="px-2.5 py-0.5 bg-muted rounded text-muted-foreground font-semibold text-xs">Employee</span>
                    <ChevronRight className="w-4 h-4 text-muted-foreground/60" />
                    <span className="px-2.5 py-0.5 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 rounded font-semibold text-xs">Manager</span>
                    <ChevronRight className="w-4 h-4 text-muted-foreground/60" />
                    <span className="px-2.5 py-0.5 bg-sky-500/10 text-sky-600 dark:text-sky-400 rounded font-semibold text-xs">Admin</span>
                    <ChevronRight className="w-4 h-4 text-muted-foreground/60" />
                    <span className="px-2.5 py-0.5 bg-violet-500/10 text-violet-600 dark:text-violet-400 rounded font-semibold text-xs">SuperAdmin</span>
                </div>
            </div>

            {/* Create Role Modal */}
            {showCreateModal && (
                <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
                    <div className="bg-popover rounded-xl shadow-xl border border-border max-w-md w-full">
                        <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
                            <h3 className="text-base font-semibold text-foreground">
                                Create Custom Role
                            </h3>
                            <button
                                onClick={() => setShowCreateModal(false)}
                                className="text-muted-foreground hover:text-foreground transition-colors"
                            >
                                <X className="w-5 h-5" />
                            </button>
                        </div>
                        <div className="p-5 space-y-4">
                            <div>
                                <label className="block text-xs font-semibold text-foreground mb-1">
                                    Role Name *
                                </label>
                                <input
                                    type="text"
                                    value={newRoleName}
                                    onChange={(e) => setNewRoleName(e.target.value)}
                                    placeholder="e.g., Senior Manager"
                                    className="w-full px-3 py-1.5 border border-border rounded-lg bg-muted/40 placeholder-muted-foreground/60 text-foreground focus:outline-none focus:ring-2 focus:ring-ring text-sm transition-all"
                                />
                            </div>
                            <div>
                                <label className="block text-xs font-semibold text-foreground mb-1">
                                    Description
                                </label>
                                <textarea
                                    value={newRoleDescription}
                                    onChange={(e) => setNewRoleDescription(e.target.value)}
                                    placeholder="What is this role for?"
                                    rows={2}
                                    className="w-full px-3 py-1.5 border border-border rounded-lg bg-muted/40 placeholder-muted-foreground/60 text-foreground focus:outline-none focus:ring-2 focus:ring-ring text-sm transition-all"
                                />
                            </div>
                            {isSuperAdmin && (
                                <div>
                                    <label className="flex items-center p-3 border border-border rounded-lg cursor-pointer hover:bg-muted/50 transition-colors">
                                        <input
                                            type="checkbox"
                                            checked={isGlobalRole}
                                            onChange={(e) => setIsGlobalRole(e.target.checked)}
                                            className="h-4 w-4 text-foreground focus:ring-ring border-border rounded"
                                        />
                                        <div className="ml-3">
                                            <span className="text-sm font-medium text-foreground flex items-center gap-2">
                                                <Globe className="w-4 h-4 text-muted-foreground" />
                                                Create as Global Role
                                            </span>
                                            <p className="text-xs text-muted-foreground">
                                                Global roles are available to all companies
                                            </p>
                                        </div>
                                    </label>
                                </div>
                            )}
                            <div>
                                <label className="block text-xs font-semibold text-foreground mb-1">
                                    Base Permission Level *
                                </label>
                                <p className="text-xs text-muted-foreground mb-2">
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
                                                        : "border-border hover:bg-muted/50"
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
                                                <span className={clsx("font-semibold text-sm", colors.text)}>
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
                        <div className="px-5 py-3.5 border-t border-border flex justify-end gap-3">
                            <button
                                onClick={() => setShowCreateModal(false)}
                                className="px-3 py-1.5 text-sm font-medium text-foreground hover:bg-muted rounded-lg transition-colors"
                            >
                                Cancel
                            </button>
                            <button
                                onClick={handleCreateRole}
                                disabled={!newRoleName.trim() || isCreating}
                                className="px-3 py-1.5 bg-foreground text-background rounded-lg hover:bg-foreground/90 text-sm font-medium disabled:opacity-50 transition-colors"
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
