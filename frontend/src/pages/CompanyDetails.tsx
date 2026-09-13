import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
    Building2,
    ArrowLeft,
    Save,
    Trash2,
    Users,
    HardDrive,
    Shield,
    Activity,
    CheckCircle,
    XCircle,
    Globe,
    Calendar,
    Settings,
    Plus,
    X,
    Lock,
    Info,
    Bell,
    Mail,
    BellRing,
    AlertTriangle,
    Ban,
    Play,
    Edit2,
    Download
} from 'lucide-react';
import { useAuthFetch, useAuth } from '../context/AuthContext';
import { useGlobalSettings } from '../context/GlobalSettingsContext';
import { useSettings, getComplianceEnforcementSummary, ComplianceMode } from '../context/SettingsContext';
import { InviteUserModal, UserData } from '../components/InviteUserModal';
import { ComplianceBadge } from '../components/ComplianceBadge';
import { LockedToggle } from '../components/LockedField';
import { TenantEmailTemplates } from '../components/TenantEmailTemplates';
import { TenantAiSettings } from '../components/TenantAiSettings';
import { BackupRestore } from '../components/BackupRestore';
import { useTranslations } from '../context/I18nContext';
import { useModalDialog } from '../context/ModalDialogContext';
import clsx from 'clsx';

interface Tenant {
    id: string;
    name: string;
    domain: string;
    plan: string;
    status: string;
    compliance_mode: string;
    data_export_enabled?: boolean;
    user_count?: number;
    storage_used_bytes?: number;
    storage_quota_bytes?: number;
    max_upload_size_bytes?: number;
    retention_policy_days?: number;
    created_at: string;
    updated_at?: string;
    smtp_host?: string;
    smtp_port?: number;
    smtp_username?: string;
    smtp_password?: string;
    smtp_from?: string;
    smtp_secure?: boolean;
    enable_totp?: boolean;
    mfa_required?: boolean;
    session_timeout_minutes?: number;
    public_sharing_enabled?: boolean;
    auth_methods?: string[];
    approval_workflow_enabled?: boolean;
    backup_enabled?: boolean;
}

interface Department {
    id: string;
    name: string;
    user_count: number;
}

export function CompanyDetails() {
    const t = useTranslations('CompanyDetails');
    const tCommon = useTranslations('Common');
    const tCompliance = useTranslations('Compliance');
    const { alert: modalAlert, confirm: modalConfirm } = useModalDialog();
    const { slug } = useParams<{ slug: string }>();

    const getLocalizedComplianceSummary = (mode: string): string[] => {
        switch (mode?.toUpperCase()) {
            case 'HIPAA':
                return [
                    tCompliance('enforceMfaAll'),
                    tCompliance('enforceSessionTimeout'),
                    tCompliance('enforcePublicDisabled'),
                    tCompliance('enforceFileAccessLogged'),
                ];
            case 'SOX':
            case 'SOC2':
                return [
                    tCompliance('enforceMfaAll'),
                    tCompliance('enforceVersioning'),
                    tCompliance('enforcePublicDisabled'),
                    tCompliance('enforceDocPermLogged'),
                    tCompliance('enforceMinRetention'),
                ];
            case 'GDPR':
                return [
                    tCompliance('enforceMfaAll'),
                    tCompliance('enforceGdprConsent'),
                    tCompliance('enforceGdprErasure'),
                    tCompliance('enforceGdprExport'),
                ];
            default:
                return [];
        }
    };
    const navigate = useNavigate();
    const authFetch = useAuthFetch();
    const { refreshUser, tenant, user: currentUser } = useAuth();
    const { formatDate } = useGlobalSettings();
    const { restrictions, canModifySetting, refreshRestrictions, setComplianceMode } = useSettings();

    const [company, setCompany] = useState<Tenant | null>(null);
    const [departments, setDepartments] = useState<Department[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [activeTab, setActiveTab] = useState<'overview' | 'settings' | 'departments' | 'users' | 'audit' | 'notifications' | 'email-templates' | 'ai' | 'document-workflow' | 'backup'>('overview');
    
    // Notification settings state
    const [notificationSettings, setNotificationSettings] = useState<any[]>([]);
    const [notificationsByRole, setNotificationsByRole] = useState<Record<string, any[]>>({});
    const [globalNotificationSettings, setGlobalNotificationSettings] = useState<any[]>([]);
    const [selectedNotificationRole, setSelectedNotificationRole] = useState<string | null>(null); // null = All Users (global)
    const [isLoadingNotifications, setIsLoadingNotifications] = useState(false);
    const [isSavingNotifications, setIsSavingNotifications] = useState(false);
    const availableRoles = ['Admin', 'Manager', 'Employee'];
    const [isSaving, setIsSaving] = useState(false);
    const [isSavingAudit, setIsSavingAudit] = useState(false);
    const [auditSettingsLocked, setAuditSettingsLocked] = useState(false);

    // Edit states
    const [editName, setEditName] = useState('');
    const [editDomain, setEditDomain] = useState('');
    const [editQuota, setEditQuota] = useState(1);
    const [editMaxUpload, setEditMaxUpload] = useState(1073741824); // 1GB default in bytes
    const [editCompliance, setEditCompliance] = useState('');
    const [editRetention, setEditRetention] = useState(30);
    const [editDataExportEnabled, setEditDataExportEnabled] = useState(true);
    const [editStatus, setEditStatus] = useState('');
    
    // Blocked extensions
    const [blockedExtensions, setBlockedExtensions] = useState<string[]>([]);
    const [newExtension, setNewExtension] = useState('');

    // Password policy
    const [passwordPolicy, setPasswordPolicy] = useState({
        min_length: 8,
        require_uppercase: true,
        require_lowercase: true,
        require_number: true,
        require_special: false,
        max_age_days: null as number | null,
        prevent_reuse: 0,
    });
    const [isSavingPasswordPolicy, setIsSavingPasswordPolicy] = useState(false);

    // IP restrictions
    const [ipRestrictions, setIpRestrictions] = useState({
        mode: 'disabled',
        allowlist: [] as string[],
        blocklist: [] as string[],
    });
    const [newAllowlistIp, setNewAllowlistIp] = useState('');
    const [newBlocklistIp, setNewBlocklistIp] = useState('');
    const [isSavingIpRestrictions, setIsSavingIpRestrictions] = useState(false);

    // SMTP states
    const [editSmtpHost, setEditSmtpHost] = useState('');
    const [editSmtpPort, setEditSmtpPort] = useState(587);
    const [editSmtpUsername, setEditSmtpUsername] = useState('');
    const [editSmtpPassword, setEditSmtpPassword] = useState('');
    const [editSmtpFrom, setEditSmtpFrom] = useState('');
    const [editSmtpSecure, setEditSmtpSecure] = useState(true);
    const [isTestingSmtp, setIsTestingSmtp] = useState(false);

    // Auth states
    const [editEnableTotp, setEditEnableTotp] = useState(false);

    // Approval workflow
    const [editApprovalWorkflow, setEditApprovalWorkflow] = useState(false);
    const [approvalPolicies, setApprovalPolicies] = useState<any[]>([]);
    const [showAddPolicy, setShowAddPolicy] = useState(false);
    const [editingPolicyId, setEditingPolicyId] = useState<string | null>(null);
    const [newPolicyName, setNewPolicyName] = useState('');
    const [newPolicyScope, setNewPolicyScope] = useState('all');
    const [newPolicyScopeValue, setNewPolicyScopeValue] = useState('');

    // Audit settings states
    const [auditLogLogins, setAuditLogLogins] = useState(true);
    const [auditLogFileOperations, setAuditLogFileOperations] = useState(true);
    const [auditLogUserChanges, setAuditLogUserChanges] = useState(true);
    const [auditLogSettingsChanges, setAuditLogSettingsChanges] = useState(true);
    const [auditLogRoleChanges, setAuditLogRoleChanges] = useState(true);
    const [auditRetentionDays, setAuditRetentionDays] = useState(90);
    
    // Check if MFA toggle is locked due to compliance
    const isMfaLocked = restrictions?.mfa_locked || false;
    const isPublicSharingLocked = restrictions?.public_sharing_locked || false;
    const isRetentionLocked = restrictions?.retention_policy_locked || false;
    const minRetentionDays = restrictions?.min_retention_days || null;

    // Department states
    const [newDeptName, setNewDeptName] = useState('');
    const [isAddingDept, setIsAddingDept] = useState(false);

    // User states
    const [users, setUsers] = useState<any[]>([]);
    const [selectedUser, setSelectedUser] = useState<any>(null);
    const [isInviteModalOpen, setIsInviteModalOpen] = useState(false);

    // Danger zone states
    const [showSuspendConfirm, setShowSuspendConfirm] = useState(false);
    const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
    const [suspendReason, setSuspendReason] = useState('');
    const [deleteConfirmName, setDeleteConfirmName] = useState('');
    const [isSuspending, setIsSuspending] = useState(false);
    const [isDeleting, setIsDeleting] = useState(false);

    useEffect(() => {
        if (slug) {
            fetchCompanyDetails();
            fetchAuditSettings();
        }
    }, [slug]);

    useEffect(() => {
        if (activeTab === 'notifications' && company?.id) {
            fetchNotificationSettings(company.id, selectedNotificationRole);
        }
    }, [activeTab, company?.id, selectedNotificationRole]);

    useEffect(() => {
        if (activeTab === 'document-workflow' && company?.id) {
            authFetch(`/api/approvals/${company.id}/policies`)
                .then(res => res.ok ? res.json() : null)
                .then(data => { if (data?.policies) setApprovalPolicies(data.policies); })
                .catch(() => {});
        }
    }, [activeTab, company?.id]);

    const fetchCompanyDetails = async () => {
        try {
            // Use different endpoint based on role
            // SuperAdmin can search all tenants, others use their accessible tenants
            const isSuperAdmin = currentUser?.role === 'SuperAdmin';
            const endpoint = isSuperAdmin 
                ? `/api/tenants?search=${encodeURIComponent(slug || '')}`
                : `/api/tenants/accessible`;
            
            const response = await authFetch(endpoint);
            if (response.ok) {
                const data = await response.json();
                // Find exact match by name
                const found = data.find((t: Tenant) => t.name === decodeURIComponent(slug || ''));
                if (found) {
                    setCompany(found);
                    setEditName(found.name);
                    setEditDomain(found.domain);
                    setEditQuota(found.storage_quota_bytes ? Math.round(found.storage_quota_bytes / (1024 * 1024 * 1024 * 1024)) : 1);
                    setEditMaxUpload(found.max_upload_size_bytes || 1073741824); // Default 1GB
                    // Normalize compliance_mode: backend uses 'Standard', frontend uses 'none' for standard mode
                    const mode = (found.compliance_mode || 'Standard').toLowerCase();
                    setEditCompliance(mode === 'standard' ? 'none' : mode);
                    setEditRetention(found.retention_policy_days || 30);
                    setEditDataExportEnabled(found.data_export_enabled !== false); // Default true
                    setEditStatus(found.status);

                    setEditSmtpHost(found.smtp_host || '');
                    setEditSmtpPort(found.smtp_port || 587);
                    setEditSmtpUsername(found.smtp_username || '');
                    setEditSmtpPassword(found.smtp_password || '');
                    setEditSmtpFrom(found.smtp_from || '');
                    setEditSmtpSecure(found.smtp_secure !== false); // Default true

                    setEditEnableTotp(found.enable_totp || false);
                    setEditApprovalWorkflow(found.approval_workflow_enabled || false);

                    // Fetch departments and users once we have the ID
                    fetchDepartments(found.id);
                    fetchUsers(found.id);
                    fetchBlockedExtensions();
                    fetchPasswordPolicy();
                    fetchIpRestrictions();
                }
            }
        } catch (error) {
            console.error('Failed to fetch company details', error);
        } finally {
            setIsLoading(false);
        }
    };

    const fetchDepartments = async (tenantId: string) => {
        try {
            const res = await authFetch(`/api/departments?tenant_id=${tenantId}`);
            if (res.ok) {
                const data = await res.json();
                setDepartments(data);
            }
        } catch (error) {
            console.error('Failed to fetch departments', error);
        }
    };

    const fetchBlockedExtensions = async () => {
        try {
            const res = await authFetch('/api/settings/blocked-extensions');
            if (res.ok) {
                const data = await res.json();
                setBlockedExtensions(data.blocked_extensions || []);
            }
        } catch (error) {
            console.error('Failed to fetch blocked extensions', error);
        }
    };

    const saveBlockedExtensions = async (extensions: string[]) => {
        try {
            await authFetch('/api/settings/blocked-extensions', {
                method: 'PUT',
                body: JSON.stringify({ blocked_extensions: extensions }),
            });
        } catch (error) {
            console.error('Failed to save blocked extensions', error);
        }
    };

    const fetchPasswordPolicy = async () => {
        try {
            const res = await authFetch('/api/settings/password-policy');
            if (res.ok) {
                const data = await res.json();
                setPasswordPolicy(data);
            }
        } catch (error) {
            console.error('Failed to fetch password policy', error);
        }
    };

    const savePasswordPolicy = async () => {
        setIsSavingPasswordPolicy(true);
        try {
            await authFetch('/api/settings/password-policy', {
                method: 'PUT',
                body: JSON.stringify(passwordPolicy),
            });
        } catch (error) {
            console.error('Failed to save password policy', error);
        } finally {
            setIsSavingPasswordPolicy(false);
        }
    };

    const fetchIpRestrictions = async () => {
        try {
            const res = await authFetch('/api/settings/ip-restrictions');
            if (res.ok) {
                const data = await res.json();
                setIpRestrictions(data);
            }
        } catch (error) {
            console.error('Failed to fetch IP restrictions', error);
        }
    };

    const saveIpRestrictions = async () => {
        setIsSavingIpRestrictions(true);
        try {
            await authFetch('/api/settings/ip-restrictions', {
                method: 'PUT',
                body: JSON.stringify(ipRestrictions),
            });
        } catch (error) {
            console.error('Failed to save IP restrictions', error);
        } finally {
            setIsSavingIpRestrictions(false);
        }
    };

    const handleSaveSettings = async () => {
        if (!company) return;
        setIsSaving(true);
        try {
            const response = await authFetch(`/api/tenants/${company.id}`, {
                method: 'PUT',
                body: JSON.stringify({
                    name: editName,
                    domain: editDomain,
                    storage_quota_bytes: editQuota * 1024 * 1024 * 1024 * 1024,
                    max_upload_size_bytes: editMaxUpload,
                    // Convert frontend 'none' to backend 'Standard' 
                    compliance_mode: editCompliance === 'none' ? 'Standard' : editCompliance.toUpperCase(),
                    retention_policy_days: editRetention,
                    data_export_enabled: editDataExportEnabled,
                    smtp_host: editSmtpHost,
                    smtp_port: editSmtpPort,
                    smtp_username: editSmtpUsername,
                    smtp_password: editSmtpPassword,
                    smtp_from: editSmtpFrom,
                    smtp_secure: editSmtpSecure,
                    enable_totp: editEnableTotp,
                    approval_workflow_enabled: editApprovalWorkflow
                }),
            });

            if (response.ok) {
                const updated = await response.json();
                setCompany({ ...company, ...updated });
                // Reload all company data to refresh UI
                await fetchCompanyDetails();
                // If this is the current tenant, immediately update global compliance state
                if (tenant && company.id === tenant.id) {
                    // Directly update the compliance mode in SettingsContext for immediate UI update
                    const newMode = editCompliance === 'none' ? 'Standard' : editCompliance.toUpperCase();
                    const mappedMode: ComplianceMode = 
                        newMode === 'HIPAA' ? 'HIPAA' :
                        newMode === 'SOX' || newMode === 'SOC2' ? 'SOX' :
                        newMode === 'GDPR' ? 'GDPR' : 'Standard';
                    setComplianceMode(mappedMode);
                    // Also refresh restrictions and user data
                    await refreshRestrictions();
                    await refreshUser();
                }
                await modalAlert({
                    title: tCommon('successTitle') || 'Success',
                    description: tCommon('savedSuccess') || 'Settings saved successfully!',
                    variant: 'success'
                });
                // If name changed, navigate to new slug
                if (editName !== company.name) {
                    navigate(`/companies/${encodeURIComponent(editName)}`, { replace: true });
                }
            } else {
                const errorText = await response.text();
                console.error('Failed to update company:', response.status, errorText);
                await modalAlert({
                    title: tCommon('errorTitle') || 'Error',
                    description: `Failed to save settings: ${response.status} ${response.statusText}`,
                    variant: 'destructive'
                });
            }
        } catch (error) {
            console.error('Failed to update company', error);
            await modalAlert({
                title: tCommon('errorTitle') || 'Error',
                description: 'Failed to save settings. Check console for details.',
                variant: 'destructive'
            });
        } finally {
            setIsSaving(false);
        }
    };

    const handleSuspendCompany = async () => {
        if (!company) return;
        setIsSuspending(true);
        try {
            const response = await authFetch(`/api/tenants/${company.id}/suspend`, {
                method: 'POST',
                body: JSON.stringify({ reason: suspendReason }),
            });

            if (response.ok) {
                setCompany({ ...company, status: 'suspended' });
                setEditStatus('suspended');
                setShowSuspendConfirm(false);
                setSuspendReason('');
                await modalAlert({
                    title: tCommon('successTitle') || 'Success',
                    description: 'Company suspended successfully',
                    variant: 'success'
                });
            } else {
                const errorText = await response.text();
                await modalAlert({
                    title: tCommon('errorTitle') || 'Error',
                    description: `Failed to suspend company: ${errorText}`,
                    variant: 'destructive'
                });
            }
        } catch (error) {
            console.error('Failed to suspend company', error);
            await modalAlert({
                title: tCommon('errorTitle') || 'Error',
                description: 'An error occurred while suspending the company.',
                variant: 'destructive'
            });
        } finally {
            setIsSuspending(false);
        }
    };

    const handleUnsuspendCompany = async () => {
        if (!company) return;
        setIsSuspending(true);
        try {
            const response = await authFetch(`/api/tenants/${company.id}/unsuspend`, {
                method: 'POST',
            });

            if (response.ok) {
                setCompany({ ...company, status: 'active' });
                setEditStatus('active');
                await modalAlert({
                    title: tCommon('successTitle') || 'Success',
                    description: 'Company unsuspended successfully',
                    variant: 'success'
                });
            } else {
                const errorText = await response.text();
                await modalAlert({
                    title: tCommon('errorTitle') || 'Error',
                    description: `Failed to unsuspend company: ${errorText}`,
                    variant: 'destructive'
                });
            }
        } catch (error) {
            console.error('Failed to unsuspend company', error);
            await modalAlert({
                title: tCommon('errorTitle') || 'Error',
                description: 'An error occurred while unsuspending the company.',
                variant: 'destructive'
            });
        } finally {
            setIsSuspending(false);
        }
    };

    const handleDeleteCompany = async () => {
        if (!company || deleteConfirmName !== company.name) return;
        setIsDeleting(true);
        try {
            const response = await authFetch(`/api/tenants/${company.id}`, {
                method: 'DELETE',
            });

            if (response.ok) {
                await modalAlert({
                    title: tCommon('successTitle') || 'Success',
                    description: 'Company deleted successfully',
                    variant: 'success'
                });
                navigate('/companies');
            } else {
                const errorText = await response.text();
                await modalAlert({
                    title: tCommon('errorTitle') || 'Error',
                    description: `Failed to delete company: ${errorText}`,
                    variant: 'destructive'
                });
            }
        } catch (error) {
            console.error('Failed to delete company', error);
            await modalAlert({
                title: tCommon('errorTitle') || 'Error',
                description: 'An error occurred while deleting the company.',
                variant: 'destructive'
            });
        } finally {
            setIsDeleting(false);
        }
    };

    const handleAddDepartment = async () => {
        if (!company || !newDeptName.trim()) return;
        try {
            const response = await authFetch(`/api/departments?tenant_id=${company.id}`, {
                method: 'POST',
                body: JSON.stringify({ name: newDeptName }),
            });

            if (response.ok) {
                setNewDeptName('');
                setIsAddingDept(false);
                fetchDepartments(company.id);
            }
        } catch (error) {
            console.error('Failed to add department', error);
        }
    };

    const handleTestSmtp = async () => {
        if (!company) return;
        setIsTestingSmtp(true);
        try {
            const response = await authFetch(`/api/tenants/${company.id}/smtp/test`, {
                method: 'POST',
                body: JSON.stringify({
                    host: editSmtpHost,
                    port: editSmtpPort,
                    username: editSmtpUsername,
                    password: editSmtpPassword,
                    secure: editSmtpSecure
                }),
            });

            if (response.ok) {
                await modalAlert({
                    title: tCommon('successTitle') || 'Success',
                    description: 'SMTP Connection Successful!',
                    variant: 'success'
                });
            } else {
                await modalAlert({
                    title: tCommon('errorTitle') || 'Error',
                    description: 'SMTP Connection Failed. Please check your settings.',
                    variant: 'destructive'
                });
            }
        } catch (error) {
            console.error('Failed to test SMTP', error);
            await modalAlert({
                title: tCommon('errorTitle') || 'Error',
                description: 'An error occurred while testing SMTP.',
                variant: 'destructive'
            });
        } finally {
            setIsTestingSmtp(false);
        }
    };

    const fetchUsers = async (tenantId: string) => {
        try {
            const res = await authFetch(`/api/users?tenant_id=${tenantId}`);
            if (res.ok) {
                const data = await res.json();
                setUsers(data);
            }
        } catch (error) {
            console.error('Failed to fetch users', error);
        }
    };

    const fetchAuditSettings = async () => {
        try {
            const res = await authFetch('/api/audit-settings');
            if (res.ok) {
                const data = await res.json();
                setAuditLogLogins(data.log_logins ?? true);
                setAuditLogFileOperations(data.log_file_operations ?? true);
                setAuditLogUserChanges(data.log_user_changes ?? true);
                setAuditLogSettingsChanges(data.log_settings_changes ?? true);
                setAuditLogRoleChanges(data.log_role_changes ?? true);
                setAuditRetentionDays(data.retention_days ?? 90);
                setAuditSettingsLocked(data.compliance_locked ?? false);
            }
        } catch (error) {
            console.error('Failed to fetch audit settings', error);
        }
    };

    const fetchNotificationSettings = async (tenantId: string, role?: string | null) => {
        setIsLoadingNotifications(true);
        try {
            const url = role 
                ? `/api/tenants/${tenantId}/notification-settings?role=${role}`
                : `/api/tenants/${tenantId}/notification-settings`;
            const res = await authFetch(url);
            if (res.ok) {
                const data = await res.json();
                if (role) {
                    // Role-specific response
                    setNotificationSettings(data.settings || []);
                } else {
                    // Full response with global and by_role
                    setGlobalNotificationSettings(data.global || []);
                    setNotificationsByRole(data.by_role || {});
                    setNotificationSettings(data.global || []);
                }
            }
        } catch (error) {
            console.error('Failed to fetch notification settings', error);
        } finally {
            setIsLoadingNotifications(false);
        }
    };

    const handleSaveNotificationSetting = async (eventType: string, field: string, value: boolean) => {
        if (!company) return;
        setIsSavingNotifications(true);
        try {
            const res = await authFetch(`/api/tenants/${company.id}/notification-settings`, {
                method: 'PUT',
                body: JSON.stringify({
                    role: selectedNotificationRole,
                    settings: [{
                        event_type: eventType,
                        [field]: value
                    }]
                })
            });
            if (res.ok) {
                const data = await res.json();
                setNotificationSettings(data.settings || data.global || []);
            }
        } catch (error) {
            console.error('Failed to save notification setting', error);
        } finally {
            setIsSavingNotifications(false);
        }
    };

    const notificationLabels: Record<string, { label: string; description: string }> = {
        file_upload: { label: t('notifFileUpload'), description: t('notifFileUploadDesc') },
        request_expiring: { label: t('notifRequestExpiring'), description: t('notifRequestExpiringDesc') },
        user_action: { label: t('notifUserAction'), description: t('notifUserActionDesc') },
        compliance_alert: { label: t('notifComplianceAlert'), description: t('notifComplianceAlertDesc') },
        storage_warning: { label: t('notifStorageWarning'), description: t('notifStorageWarningDesc') },
        file_shared: { label: t('notifFileShared'), description: t('notifFileSharedDesc') }
    };

    const handleSaveAuditSettings = async () => {
        setIsSavingAudit(true);
        try {
            const response = await authFetch('/api/audit-settings', {
                method: 'PUT',
                body: JSON.stringify({
                    log_logins: auditLogLogins,
                    log_file_operations: auditLogFileOperations,
                    log_user_changes: auditLogUserChanges,
                    log_settings_changes: auditLogSettingsChanges,
                    log_role_changes: auditLogRoleChanges,
                    retention_days: auditRetentionDays,
                }),
            });

            if (response.ok) {
                await modalAlert({
                    title: tCommon('successTitle') || 'Success',
                    description: 'Audit settings saved successfully!',
                    variant: 'success'
                });
            } else {
                await modalAlert({
                    title: tCommon('errorTitle') || 'Error',
                    description: 'Failed to save audit settings.',
                    variant: 'destructive'
                });
            }
        } catch (error) {
            console.error('Failed to save audit settings', error);
            await modalAlert({
                title: tCommon('errorTitle') || 'Error',
                description: 'An error occurred while saving audit settings.',
                variant: 'destructive'
            });
        } finally {
            setIsSavingAudit(false);
        }
    };

    const handleUserSubmit = async (data: UserData) => {
        if (!company) return;

        try {
            let response;
            if (selectedUser) {
                // Update existing user
                response = await authFetch(`/api/users/${selectedUser.id}`, {
                    method: 'PUT',
                    body: JSON.stringify({
                        name: data.name,
                        role: data.role,
                        department_id: data.department_id || null,
                        allowed_tenant_ids: data.allowed_tenant_ids,
                    }),
                });
            } else {
                // Create new user
                response = await authFetch('/api/users', {
                    method: 'POST',
                    body: JSON.stringify({
                        ...data,
                        tenant_id: company.id
                    }),
                });
            }

            if (response.ok) {
                setIsInviteModalOpen(false);
                setSelectedUser(null);
                fetchUsers(company.id);
                // Refresh company details to update user count
                fetchCompanyDetails();
            }
        } catch (error) {
            console.error('Failed to save user', error);
        }
    };

    const handleEditUser = (user: any) => {
        setSelectedUser(user);
        setIsInviteModalOpen(true);
    };

    const formatBytes = (bytes?: number) => {
        if (!bytes) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    };

    if (isLoading) {
        return (
            <div className="flex items-center justify-center h-96">
                <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-600"></div>
            </div>
        );
    }

    if (!company) {
        return (
            <div className="text-center py-12">
                <h2 className="text-xl font-semibold text-gray-900 dark:text-white">{tCommon('noData') || 'Company not found'}</h2>
                <button onClick={() => navigate('/companies')} className="mt-4 text-primary-600 hover:underline">
                    {tCommon('back') || 'Back'}
                </button>
            </div>
        );
    }

    const getStatusBadgeText = (status: string) => {
        if (status === 'active') return t('statusActive');
        if (status === 'suspended') return t('statusSuspended');
        if (status === 'archived') return t('statusArchived');
        if (status === 'trial') return t('statusTrial');
        return status.charAt(0).toUpperCase() + status.slice(1);
    };

    const getTabTitle = (tab: string) => {
        switch (tab) {
            case 'overview': return t('tabOverview');
            case 'settings': return t('tabSettings');
            case 'departments': return t('tabDepartments');
            case 'users': return t('tabUsers');
            case 'document-workflow': return t('tabDocumentWorkflow');
            case 'notifications': return t('tabNotifications');
            case 'email-templates': return t('tabEmailTemplates');
            case 'ai': return t('tabAi');
            case 'audit': return t('tabAudit');
            case 'backup': return t('tabBackup');
            default: return tab.charAt(0).toUpperCase() + tab.slice(1);
        }
    };

    return (
        <div className="space-y-6">
            {/* Header */}
            <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
                <div className="flex items-center gap-4">
                    <button
                        onClick={() => navigate('/companies')}
                        className="p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-colors"
                    >
                        <ArrowLeft className="w-5 h-5 text-gray-500" />
                    </button>
                    <div className="h-12 w-12 rounded-xl bg-gradient-to-br from-primary-500 to-primary-600 flex items-center justify-center text-white shadow-lg">
                        <Building2 className="w-6 h-6" />
                    </div>
                    <div>
                        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">{company.name}</h1>
                        <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                            <Globe className="w-3 h-3" />
                            {company.domain}
                            <span className="mx-1">•</span>
                            <span className={clsx(
                                "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium",
                                company.status === 'active'
                                    ? "bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-400"
                                    : "bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-400"
                            )}>
                                {getStatusBadgeText(company.status)}
                            </span>
                        </div>
                    </div>
                </div>
                <div className="flex items-center gap-3">
                    <button
                        onClick={() => setActiveTab('settings')}
                        className="px-4 py-2 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 text-gray-700 dark:text-gray-200 rounded-lg text-sm font-medium hover:bg-gray-50 dark:hover:bg-gray-700 shadow-sm transition-all"
                    >
                        {t('editDetails')}
                    </button>
                </div>
            </div>

            {/* Tabs */}
            <div className="border-b border-gray-200 dark:border-gray-700">
                <nav className="-mb-px flex space-x-8 overflow-x-auto">
                    {['overview', 'settings', 'departments', 'users', 'document-workflow', 'notifications', 'email-templates', 'ai', 'audit', 'backup'].filter((tab) => {
                        if (tab === 'backup' && company?.backup_enabled === false && currentUser?.role !== 'SuperAdmin') return false;
                        return true;
                    }).map((tab) => (
                        <button
                            key={tab}
                            onClick={() => setActiveTab(tab as any)}
                            className={clsx(
                                "whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm transition-colors",
                                activeTab === tab
                                    ? "border-primary-500 text-primary-600 dark:text-primary-400"
                                    : "border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300"
                            )}
                        >
                            {getTabTitle(tab)}
                        </button>
                    ))}
                </nav>
            </div>

            {/* Content */}
            <div className="min-h-[400px]">
                {activeTab === 'overview' && (
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
                        {/* Stats Cards */}
                        <div className="bg-white dark:bg-gray-800 p-6 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                            <div className="flex items-center justify-between mb-4">
                                <h3 className="text-sm font-medium text-gray-500 dark:text-gray-400">{t('totalUsers')}</h3>
                                <Users className="w-5 h-5 text-primary-500" />
                            </div>
                            <div className="text-3xl font-bold text-gray-900 dark:text-white">{company.user_count || 0}</div>
                            <p className="text-xs text-gray-500 mt-1">{t('activeAccounts')}</p>
                        </div>

                        <div className="bg-white dark:bg-gray-800 p-6 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                            <div className="flex items-center justify-between mb-4">
                                <h3 className="text-sm font-medium text-gray-500 dark:text-gray-400">{t('storageUsed')}</h3>
                                <HardDrive className="w-5 h-5 text-blue-500" />
                            </div>
                            <div className="text-3xl font-bold text-gray-900 dark:text-white">{formatBytes(company.storage_used_bytes || 0)}</div>
                            <div className="w-full bg-gray-200 dark:bg-gray-700 rounded-full h-1.5 mt-3">
                                <div
                                    className="bg-blue-500 h-1.5 rounded-full"
                                    style={{ width: `${Math.min(((company.storage_used_bytes || 0) / (company.storage_quota_bytes || 1)) * 100, 100)}%` }}
                                ></div>
                            </div>
                            <p className="text-xs text-gray-500 mt-1">{t('ofQuota', { quota: formatBytes(company.storage_quota_bytes) })}</p>
                        </div>

                        <div className="bg-white dark:bg-gray-800 p-6 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                            <div className="flex items-center justify-between mb-4">
                                <h3 className="text-sm font-medium text-gray-500 dark:text-gray-400">{t('compliance')}</h3>
                                <Shield className="w-5 h-5 text-green-500" />
                            </div>
                            <div className="text-3xl font-bold text-gray-900 dark:text-white">
                                {(() => {
                                    const mode = company.compliance_mode?.toUpperCase();
                                    if (mode === 'HIPAA') return 'HIPAA';
                                    if (mode === 'GDPR') return 'GDPR';
                                    if (mode === 'SOX' || mode === 'SOC2') return 'SOX';
                                    return 'Standard';
                                })()}
                            </div>
                            <p className="text-xs text-gray-500 mt-1">{t('currentMode')}</p>
                        </div>

                        {/* Details Section */}
                        <div className="md:col-span-2 bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
                            <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                                <h3 className="text-lg font-medium text-gray-900 dark:text-white">{t('companyInfo')}</h3>
                            </div>
                            <div className="p-6 grid grid-cols-1 md:grid-cols-2 gap-6">
                                <div>
                                    <label className="text-xs font-medium text-gray-500 uppercase tracking-wider">{t('companyName')}</label>
                                    <p className="mt-1 text-sm font-medium text-gray-900 dark:text-white">{company.name}</p>
                                </div>
                                <div>
                                    <label className="text-xs font-medium text-gray-500 uppercase tracking-wider">{t('domain')}</label>
                                    <p className="mt-1 text-sm font-medium text-gray-900 dark:text-white">{company.domain}</p>
                                </div>
                                <div>
                                    <label className="text-xs font-medium text-gray-500 uppercase tracking-wider">{t('createdAt')}</label>
                                    <p className="mt-1 text-sm font-medium text-gray-900 dark:text-white">
                                        {formatDate(company.created_at)}
                                    </p>
                                </div>
                                <div>
                                    <label className="text-xs font-medium text-gray-500 uppercase tracking-wider">{t('status')}</label>
                                    <div className="mt-1">
                                        <span className={clsx(
                                            "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium",
                                            company.status === 'active'
                                                ? "bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300"
                                                : "bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-300"
                                        )}>
                                            {getStatusBadgeText(company.status).toUpperCase()}
                                        </span>
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                )}

                {activeTab === 'settings' && (
                    <div className="max-w-6xl mx-auto space-y-6">
                        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
                        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                            <h3 className="text-lg font-medium text-gray-900 dark:text-white">{t('generalSettings')}</h3>
                        </div>
                        <div className="p-6 space-y-6">
                            <div>
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('companyName')}</label>
                                <input
                                    type="text"
                                    value={editName}
                                    onChange={(e) => setEditName(e.target.value)}
                                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                />
                            </div>

                            <div>
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('domain')}</label>
                                <input
                                    type="text"
                                    value={editDomain}
                                    onChange={(e) => setEditDomain(e.target.value)}
                                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                />
                            </div>

                            <div>
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('storageQuotaGb')}</label>
                                <input
                                    type="number"
                                    min="1"
                                    max="100"
                                    value={editQuota}
                                    onChange={(e) => setEditQuota(Number(e.target.value))}
                                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                />
                            </div>

                            <div>
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('maxUploadSize')}</label>
                                <p className="text-xs text-gray-500 dark:text-gray-400 mb-2">
                                    {t('storageAndUploadDesc')}
                                </p>
                                <div className="flex flex-wrap gap-2">
                                    {[
                                        { value: 104857600, label: '100 MB' },
                                        { value: 262144000, label: '250 MB' },
                                        { value: 524288000, label: '500 MB' },
                                        { value: 1073741824, label: '1 GB' },
                                        { value: 2147483648, label: '2 GB' },
                                        { value: 5368709120, label: '5 GB' },
                                    ].map(({ value, label }) => (
                                        <button
                                            key={value}
                                            type="button"
                                            onClick={() => setEditMaxUpload(value)}
                                            className={clsx(
                                                "px-3 py-1.5 text-sm rounded-lg border transition-colors",
                                                editMaxUpload === value
                                                    ? "bg-primary-100 dark:bg-primary-900/30 border-primary-500 text-primary-700 dark:text-primary-300"
                                                    : "bg-white dark:bg-gray-700 border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:border-primary-300"
                                            )}
                                        >
                                            {label}
                                        </button>
                                    ))}
                                </div>
                            </div>

                            <div>
                                <div className="flex items-center justify-between mb-1">
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">{t('complianceMode')}</label>
                                    {editCompliance && editCompliance !== 'none' && editCompliance !== 'Standard' && (
                                        <ComplianceBadge mode={editCompliance} size="sm" />
                                    )}
                                </div>
                                <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                                    {['none', 'hipaa', 'gdpr', 'sox'].map((mode) => (
                                        <div
                                            key={mode}
                                            onClick={() => setEditCompliance(mode)}
                                            className={clsx(
                                                "relative rounded-lg border p-4 cursor-pointer flex flex-col hover:border-primary-300 dark:hover:border-primary-500 transition-colors",
                                                editCompliance === mode ? "bg-primary-50 dark:bg-primary-900/20 border-primary-500 ring-1 ring-primary-500" : "border-gray-300 dark:border-gray-600"
                                            )}
                                        >
                                            <div className="flex items-center justify-between">
                                                <span className="block text-sm font-medium text-gray-900 dark:text-white">
                                                    {mode === 'none' ? t('modeStandard') : mode.toUpperCase()}
                                                </span>
                                                {editCompliance === mode && <CheckCircle className="h-5 w-5 text-primary-600 dark:text-primary-400" />}
                                            </div>
                                            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                                                {mode === 'hipaa' && t('complianceHipaaDesc')}
                                                {mode === 'sox' && t('complianceSoxDesc')}
                                                {mode === 'gdpr' && t('complianceGdprDesc')}
                                                {mode === 'none' && t('complianceStandardDesc')}
                                            </p>
                                        </div>
                                    ))}
                                </div>
                                {/* Enforcement Summary */}
                                {editCompliance && editCompliance !== 'none' && editCompliance !== 'Standard' && (
                                    <div className="mt-4 p-4 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700">
                                        <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2 flex items-center gap-2">
                                            <Info className="w-4 h-4" />
                                            {tCompliance('controlsEnforced', { mode: editCompliance.toUpperCase() })}
                                        </h4>
                                        <ul className="space-y-1">
                                            {getLocalizedComplianceSummary(editCompliance).map((item, idx) => (
                                                <li key={idx} className="text-xs text-gray-600 dark:text-gray-400 flex items-start gap-2">
                                                    <CheckCircle className="w-3 h-3 text-green-500 mt-0.5 flex-shrink-0" />
                                                    {item}
                                                </li>
                                            ))}
                                        </ul>
                                    </div>
                                )}
                            </div>

                            <div>
                                <div className="flex items-center justify-between mb-1">
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">{t('retentionDays')}</label>
                                    {minRetentionDays && (
                                        <span className="text-xs text-amber-600 dark:text-amber-400 flex items-center gap-1">
                                            <Lock className="w-3 h-3" />
                                            {t('minDaysRequired', { days: minRetentionDays })}
                                        </span>
                                    )}
                                </div>
                                <p className="text-xs text-gray-500 dark:text-gray-400 mb-2">
                                    {t('retentionDaysDesc')}
                                </p>
                                <div className="grid grid-cols-2 gap-4 sm:grid-cols-6">
                                    {[
                                        { value: 30, label: t('daysCount', { count: 30 }) },
                                        { value: 90, label: t('daysCount', { count: 90 }) },
                                        { value: 365, label: t('yearsCount', { count: 1 }) },
                                        { value: 2190, label: t('yearsCount', { count: 6 }) },
                                        { value: 2555, label: t('yearsCount', { count: 7 }) },
                                        { value: 0, label: tCommon('never') || 'Never' },
                                    ].map(({ value, label }) => {
                                        // 0 (Never) is always allowed - it's the most conservative option
                                        const isDisabled = value !== 0 && minRetentionDays ? value < minRetentionDays : false;
                                        return (
                                        <div
                                            key={value}
                                            onClick={() => !isDisabled && setEditRetention(value)}
                                            className={clsx(
                                                "relative rounded-lg border p-3 flex flex-col items-center justify-center transition-colors",
                                                isDisabled 
                                                    ? "cursor-not-allowed opacity-50 border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800"
                                                    : "cursor-pointer hover:border-primary-300 dark:hover:border-primary-500",
                                                editRetention === value && !isDisabled
                                                    ? "bg-primary-50 dark:bg-primary-900/20 border-primary-500 ring-1 ring-primary-500"
                                                    : "border-gray-300 dark:border-gray-600"
                                            )}
                                        >
                                            <span className={clsx(
                                                "text-sm font-bold",
                                                isDisabled ? "text-gray-400 dark:text-gray-600" : "text-gray-900 dark:text-white"
                                            )}>{label}</span>
                                            {editRetention === value && !isDisabled && (
                                                <div className="absolute top-1 right-1">
                                                    <CheckCircle className="h-3 w-3 text-primary-600 dark:text-primary-400" />
                                                </div>
                                            )}
                                            {isDisabled && (
                                                <div className="absolute top-1 right-1">
                                                    <Lock className="h-3 w-3 text-gray-400 dark:text-gray-600" />
                                                </div>
                                            )}
                                        </div>
                                    )})}
                                </div>
                            </div>

                            {/* Data Export Toggle */}
                            <div className="flex items-center justify-between p-4 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700">
                                <div>
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">{t('dataExport')}</label>
                                    <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                                        {t('dataExportDesc')}
                                    </p>
                                </div>
                                <button
                                    onClick={() => setEditDataExportEnabled(!editDataExportEnabled)}
                                    className={clsx(
                                        "relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2",
                                        editDataExportEnabled
                                            ? "bg-primary-600"
                                            : "bg-gray-300 dark:bg-gray-600"
                                    )}
                                >
                                    <span
                                        className={clsx(
                                            "inline-block h-4 w-4 transform rounded-full bg-white transition-transform",
                                            editDataExportEnabled ? "translate-x-6" : "translate-x-1"
                                        )}
                                    />
                                </button>
                            </div>

                            {/* Blocked File Extensions */}
                            <div className="border-t border-gray-200 dark:border-gray-700 pt-6">
                                <h4 className="text-md font-medium text-gray-900 dark:text-white mb-2">{t('blockedExtensions')}</h4>
                                <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
                                    {t('blockedExtensionsDesc')}
                                </p>
                                <div className="flex flex-wrap gap-2 mb-3">
                                    {blockedExtensions.map((ext, index) => (
                                        <span
                                            key={index}
                                            className="inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-300"
                                        >
                                            .{ext}
                                            <button
                                                onClick={() => {
                                                    const updated = blockedExtensions.filter((_, i) => i !== index);
                                                    setBlockedExtensions(updated);
                                                    saveBlockedExtensions(updated);
                                                }}
                                                className="ml-2 hover:text-red-600 dark:hover:text-red-200"
                                            >
                                                <X className="w-3 h-3" />
                                            </button>
                                        </span>
                                    ))}
                                    {blockedExtensions.length === 0 && (
                                        <span className="text-sm text-gray-400 dark:text-gray-500 italic">{t('noBlockedExtensions')}</span>
                                    )}
                                </div>
                                <div className="flex gap-2">
                                    <input
                                        type="text"
                                        value={newExtension}
                                        onChange={(e) => setNewExtension(e.target.value.replace(/[^a-zA-Z0-9]/g, '').toLowerCase())}
                                        onKeyDown={(e) => {
                                            if (e.key === 'Enter' && newExtension) {
                                                e.preventDefault();
                                                if (!blockedExtensions.includes(newExtension)) {
                                                    const updated = [...blockedExtensions, newExtension];
                                                    setBlockedExtensions(updated);
                                                    saveBlockedExtensions(updated);
                                                }
                                                setNewExtension('');
                                            }
                                        }}
                                        placeholder={t('addExtensionPlaceholder')}
                                        className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm"
                                    />
                                    <button
                                        onClick={() => {
                                            if (newExtension && !blockedExtensions.includes(newExtension)) {
                                                const updated = [...blockedExtensions, newExtension];
                                                setBlockedExtensions(updated);
                                                saveBlockedExtensions(updated);
                                                setNewExtension('');
                                            }
                                        }}
                                        className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 text-sm font-medium"
                                    >
                                        {t('addExtension')}
                                    </button>
                                </div>
                                <div className="mt-3 flex flex-wrap gap-2">
                                    <span className="text-xs text-gray-500 dark:text-gray-400 mr-2">{t('quickAdd')}</span>
                                    {['exe', 'bat', 'sh', 'cmd', 'msi', 'dll', 'scr', 'js', 'vbs', 'ps1'].map(ext => (
                                        !blockedExtensions.includes(ext) && (
                                            <button
                                                key={ext}
                                                onClick={() => {
                                                    const updated = [...blockedExtensions, ext];
                                                    setBlockedExtensions(updated);
                                                    saveBlockedExtensions(updated);
                                                }}
                                                className="px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-300 rounded hover:bg-gray-200 dark:hover:bg-gray-600"
                                            >
                                                +.{ext}
                                            </button>
                                        )
                                    ))}
                                </div>
                            </div>

                            {/* Password Policy Section */}
                            <div className="border-t border-gray-200 dark:border-gray-700 pt-6">
                                <div className="flex items-center justify-between mb-4">
                                    <div>
                                        <h4 className="text-md font-medium text-gray-900 dark:text-white">{t('passwordPolicy')}</h4>
                                        <p className="text-sm text-gray-500 dark:text-gray-400">
                                            {t('passwordPolicyDesc')}
                                        </p>
                                    </div>
                                    <button
                                        onClick={savePasswordPolicy}
                                        disabled={isSavingPasswordPolicy}
                                        className="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 text-sm font-medium disabled:opacity-50 flex items-center gap-2"
                                    >
                                        {isSavingPasswordPolicy ? (
                                            <>
                                                <span className="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full" />
                                                {t('savingPasswordPolicy')}
                                            </>
                                        ) : (
                                            <>
                                                <Save className="w-4 h-4" />
                                                {t('savePasswordPolicy')}
                                            </>
                                        )}
                                    </button>
                                </div>
                                
                                <div className="space-y-4">
                                    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                                        <div>
                                            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                                {t('minLength')}
                                            </label>
                                            <input
                                                type="number"
                                                min={4}
                                                max={128}
                                                value={passwordPolicy.min_length}
                                                onChange={(e) => setPasswordPolicy({ ...passwordPolicy, min_length: parseInt(e.target.value) || 8 })}
                                                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                            />
                                        </div>
                                        <div>
                                            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                                {t('preventReuse')}
                                            </label>
                                            <select
                                                value={passwordPolicy.prevent_reuse}
                                                onChange={(e) => setPasswordPolicy({ ...passwordPolicy, prevent_reuse: parseInt(e.target.value) })}
                                                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                            >
                                                <option value={0}>{t('ipDisabled') || 'Disabled'}</option>
                                                <option value={3}>{t('lastPasswords', { count: 3 })}</option>
                                                <option value={6}>{t('lastPasswords', { count: 6 })}</option>
                                                <option value={12}>{t('lastPasswords', { count: 12 })}</option>
                                                <option value={24}>{t('lastPasswords', { count: 24 })}</option>
                                            </select>
                                        </div>
                                    </div>
                                    
                                    <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
                                        <label className="flex items-center p-3 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700 cursor-pointer">
                                            <input
                                                type="checkbox"
                                                checked={passwordPolicy.require_uppercase}
                                                onChange={(e) => setPasswordPolicy({ ...passwordPolicy, require_uppercase: e.target.checked })}
                                                className="form-checkbox h-4 w-4 text-primary-600 rounded border-gray-300 dark:border-gray-600"
                                            />
                                            <span className="ml-2 text-sm text-gray-700 dark:text-gray-300">{t('requireUppercase')}</span>
                                        </label>
                                        <label className="flex items-center p-3 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700 cursor-pointer">
                                            <input
                                                type="checkbox"
                                                checked={passwordPolicy.require_lowercase}
                                                onChange={(e) => setPasswordPolicy({ ...passwordPolicy, require_lowercase: e.target.checked })}
                                                className="form-checkbox h-4 w-4 text-primary-600 rounded border-gray-300 dark:border-gray-600"
                                            />
                                            <span className="ml-2 text-sm text-gray-700 dark:text-gray-300">{t('requireLowercase')}</span>
                                        </label>
                                        <label className="flex items-center p-3 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700 cursor-pointer">
                                            <input
                                                type="checkbox"
                                                checked={passwordPolicy.require_number}
                                                onChange={(e) => setPasswordPolicy({ ...passwordPolicy, require_number: e.target.checked })}
                                                className="form-checkbox h-4 w-4 text-primary-600 rounded border-gray-300 dark:border-gray-600"
                                            />
                                            <span className="ml-2 text-sm text-gray-700 dark:text-gray-300">{t('requireNumbers')}</span>
                                        </label>
                                        <label className="flex items-center p-3 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700 cursor-pointer">
                                            <input
                                                type="checkbox"
                                                checked={passwordPolicy.require_special}
                                                onChange={(e) => setPasswordPolicy({ ...passwordPolicy, require_special: e.target.checked })}
                                                className="form-checkbox h-4 w-4 text-primary-600 rounded border-gray-300 dark:border-gray-600"
                                            />
                                            <span className="ml-2 text-sm text-gray-700 dark:text-gray-300">{t('requireSpecial')}</span>
                                        </label>
                                    </div>
                                    
                                    <div className="p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
                                        <p className="text-sm text-blue-700 dark:text-blue-300">
                                            <strong>{t('currentRequirements')}</strong> {t('charsMin', { count: passwordPolicy.min_length })}
                                            {passwordPolicy.require_uppercase && t('reqUppercase')}
                                            {passwordPolicy.require_lowercase && t('reqLowercase')}
                                            {passwordPolicy.require_number && t('reqNumber')}
                                            {passwordPolicy.require_special && t('reqSpecial')}
                                        </p>
                                    </div>
                                </div>
                            </div>

                            {/* IP Restrictions Section */}
                            <div className="border-t border-gray-200 dark:border-gray-700 pt-6">
                                <div className="flex items-center justify-between mb-4">
                                    <div>
                                        <h4 className="text-md font-medium text-gray-900 dark:text-white">{t('ipRestrictions')}</h4>
                                        <p className="text-sm text-gray-500 dark:text-gray-400">
                                            {t('ipRestrictionsDesc')}
                                        </p>
                                    </div>
                                    <button
                                        onClick={saveIpRestrictions}
                                        disabled={isSavingIpRestrictions}
                                        className="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 text-sm font-medium disabled:opacity-50 flex items-center gap-2"
                                    >
                                        {isSavingIpRestrictions ? (
                                            <>
                                                <span className="animate-spin h-4 w-4 border-2 border-white border-t-transparent rounded-full" />
                                                {t('savingIpRestrictions')}
                                            </>
                                        ) : (
                                            <>
                                                <Save className="w-4 h-4" />
                                                {t('saveIpRestrictions')}
                                            </>
                                        )}
                                    </button>
                                </div>
                                
                                <div className="space-y-4">
                                    <div>
                                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                            {t('ipRestrictionMode')}
                                        </label>
                                        <select
                                            value={ipRestrictions.mode}
                                            onChange={(e) => setIpRestrictions({ ...ipRestrictions, mode: e.target.value })}
                                            className="w-full max-w-xs px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                        >
                                            <option value="disabled">{t('ipDisabled')}</option>
                                            <option value="allowlist_only">{t('ipAllowlistOnly')}</option>
                                            <option value="blocklist_only">{t('ipBlocklistOnly')}</option>
                                            <option value="both">{t('ipBoth')}</option>
                                        </select>
                                    </div>
                                    
                                    {ipRestrictions.mode !== 'disabled' && (
                                        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                                            {/* Allowlist */}
                                            {(ipRestrictions.mode === 'allowlist_only' || ipRestrictions.mode === 'both') && (
                                                <div>
                                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                                        {t('allowlistIps')}
                                                    </label>
                                                    <div className="space-y-2">
                                                        {ipRestrictions.allowlist.map((ip, index) => (
                                                            <div key={index} className="flex items-center gap-2 p-2 bg-green-50 dark:bg-green-900/20 rounded border border-green-200 dark:border-green-800">
                                                                <span className="flex-1 text-sm text-green-800 dark:text-green-300 font-mono">{ip}</span>
                                                                <button
                                                                    onClick={() => {
                                                                        const updated = ipRestrictions.allowlist.filter((_, i) => i !== index);
                                                                        setIpRestrictions({ ...ipRestrictions, allowlist: updated });
                                                                    }}
                                                                    className="text-green-600 hover:text-green-800 dark:text-green-400 dark:hover:text-green-200"
                                                                >
                                                                    <X className="w-4 h-4" />
                                                                </button>
                                                            </div>
                                                        ))}
                                                        {ipRestrictions.allowlist.length === 0 && (
                                                            <p className="text-sm text-gray-400 italic">{t('noIpsConfigured')}</p>
                                                        )}
                                                        <div className="flex gap-2">
                                                            <input
                                                                type="text"
                                                                value={newAllowlistIp}
                                                                onChange={(e) => setNewAllowlistIp(e.target.value)}
                                                                placeholder={t('addIpPlaceholder')}
                                                                className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-green-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm font-mono"
                                                                onKeyDown={(e) => {
                                                                    if (e.key === 'Enter' && newAllowlistIp.trim()) {
                                                                        e.preventDefault();
                                                                        if (!ipRestrictions.allowlist.includes(newAllowlistIp.trim())) {
                                                                            setIpRestrictions({
                                                                                ...ipRestrictions,
                                                                                allowlist: [...ipRestrictions.allowlist, newAllowlistIp.trim()]
                                                                            });
                                                                        }
                                                                        setNewAllowlistIp('');
                                                                    }
                                                                }}
                                                            />
                                                            <button
                                                                onClick={() => {
                                                                    if (newAllowlistIp.trim() && !ipRestrictions.allowlist.includes(newAllowlistIp.trim())) {
                                                                        setIpRestrictions({
                                                                            ...ipRestrictions,
                                                                            allowlist: [...ipRestrictions.allowlist, newAllowlistIp.trim()]
                                                                        });
                                                                        setNewAllowlistIp('');
                                                                    }
                                                                }}
                                                                className="px-3 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 text-sm"
                                                            >
                                                                <Plus className="w-4 h-4" />
                                                            </button>
                                                        </div>
                                                    </div>
                                                </div>
                                            )}

                                            {/* Blocklist */}
                                            {(ipRestrictions.mode === 'blocklist_only' || ipRestrictions.mode === 'both') && (
                                                <div>
                                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                                        {t('blocklistIps')}
                                                    </label>
                                                    <div className="space-y-2">
                                                        {ipRestrictions.blocklist.map((ip, index) => (
                                                            <div key={index} className="flex items-center gap-2 p-2 bg-red-50 dark:bg-red-900/20 rounded border border-red-200 dark:border-red-800">
                                                                <span className="flex-1 text-sm text-red-800 dark:text-red-300 font-mono">{ip}</span>
                                                                <button
                                                                    onClick={() => {
                                                                        const updated = ipRestrictions.blocklist.filter((_, i) => i !== index);
                                                                        setIpRestrictions({ ...ipRestrictions, blocklist: updated });
                                                                    }}
                                                                    className="text-red-600 hover:text-red-800 dark:text-red-400 dark:hover:text-red-200"
                                                                >
                                                                    <X className="w-4 h-4" />
                                                                </button>
                                                            </div>
                                                        ))}
                                                        {ipRestrictions.blocklist.length === 0 && (
                                                            <p className="text-sm text-gray-400 italic">{t('noIpsConfigured')}</p>
                                                        )}
                                                        <div className="flex gap-2">
                                                            <input
                                                                type="text"
                                                                value={newBlocklistIp}
                                                                onChange={(e) => setNewBlocklistIp(e.target.value)}
                                                                placeholder={t('addIpPlaceholder')}
                                                                className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-red-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm font-mono"
                                                                onKeyDown={(e) => {
                                                                    if (e.key === 'Enter' && newBlocklistIp.trim()) {
                                                                        e.preventDefault();
                                                                        if (!ipRestrictions.blocklist.includes(newBlocklistIp.trim())) {
                                                                            setIpRestrictions({
                                                                                ...ipRestrictions,
                                                                                blocklist: [...ipRestrictions.blocklist, newBlocklistIp.trim()]
                                                                            });
                                                                        }
                                                                        setNewBlocklistIp('');
                                                                    }
                                                                }}
                                                            />
                                                            <button
                                                                onClick={() => {
                                                                    if (newBlocklistIp.trim() && !ipRestrictions.blocklist.includes(newBlocklistIp.trim())) {
                                                                        setIpRestrictions({
                                                                            ...ipRestrictions,
                                                                            blocklist: [...ipRestrictions.blocklist, newBlocklistIp.trim()]
                                                                        });
                                                                        setNewBlocklistIp('');
                                                                    }
                                                                }}
                                                                className="px-3 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 text-sm"
                                                            >
                                                                <Plus className="w-4 h-4" />
                                                            </button>
                                                        </div>
                                                    </div>
                                                </div>
                                            )}
                                        </div>
                                    )}
                                    
                                    {ipRestrictions.mode === 'disabled' && (
                                        <div className="p-3 bg-gray-50 dark:bg-gray-900/50 rounded-lg border border-gray-200 dark:border-gray-700">
                                            <p className="text-sm text-gray-500 dark:text-gray-400">
                                                {t('ipDisabledDesc')}
                                            </p>
                                        </div>
                                    )}
                                    
                                    {ipRestrictions.mode !== 'disabled' && (
                                        <div className="p-3 bg-amber-50 dark:bg-amber-900/20 rounded-lg border border-amber-200 dark:border-amber-800">
                                            <p className="text-sm text-amber-700 dark:text-amber-300">
                                                <AlertTriangle className="w-4 h-4 inline mr-1" />
                                                <strong>{tCommon('warning') || 'Warning'}:</strong> {t('ipWarningDesc')}
                                            </p>
                                        </div>
                                    )}
                                </div>
                            </div>

                            <div>
                                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('encryptionStandard')}</label>
                                <div className="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-4 border border-gray-200 dark:border-gray-600 flex items-center">
                                    <Shield className="h-5 w-5 text-green-600 dark:text-green-400 mr-3" />
                                    <div>
                                        <p className="text-sm font-medium text-gray-900 dark:text-white">{t('encryptionAes')}</p>
                                        <p className="text-xs text-gray-500 dark:text-gray-400">{t('encryptionDesc')}</p>
                                    </div>
                                </div>
                            </div>

                            <div className="border-t border-gray-200 dark:border-gray-700 pt-6">
                                <h4 className="text-md font-medium text-gray-900 dark:text-white mb-4">{t('smtpSettings')}</h4>
                                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                                    <div className="md:col-span-2">
                                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('smtpHost')}</label>
                                        <input
                                            type="text"
                                            value={editSmtpHost}
                                            onChange={(e) => setEditSmtpHost(e.target.value)}
                                            placeholder="smtp.example.com"
                                            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                        />
                                    </div>
                                    <div>
                                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('smtpPort')}</label>
                                        <input
                                            type="number"
                                            value={editSmtpPort}
                                            onChange={(e) => setEditSmtpPort(Number(e.target.value))}
                                            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                        />
                                    </div>
                                    <div className="flex items-center pt-6">
                                        <label className="flex items-center cursor-pointer">
                                            <input
                                                type="checkbox"
                                                checked={editSmtpSecure}
                                                onChange={(e) => setEditSmtpSecure(e.target.checked)}
                                                className="form-checkbox h-5 w-5 text-primary-600 rounded border-gray-300 dark:border-gray-600 dark:bg-gray-700"
                                            />
                                            <span className="ml-2 text-sm text-gray-700 dark:text-gray-300">{t('smtpSecure')}</span>
                                        </label>
                                    </div>
                                    <div>
                                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('smtpUsername')}</label>
                                        <input
                                            type="text"
                                            value={editSmtpUsername}
                                            onChange={(e) => setEditSmtpUsername(e.target.value)}
                                            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                        />
                                    </div>
                                    <div>
                                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('smtpPassword')}</label>
                                        <input
                                            type="password"
                                            value={editSmtpPassword}
                                            onChange={(e) => setEditSmtpPassword(e.target.value)}
                                            placeholder={t('smtpPasswordPlaceholder')}
                                            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                        />
                                    </div>
                                    <div className="md:col-span-2">
                                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('smtpFrom')}</label>
                                        <input
                                            type="email"
                                            value={editSmtpFrom}
                                            onChange={(e) => setEditSmtpFrom(e.target.value)}
                                            placeholder="noreply@example.com"
                                            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                        />
                                    </div>
                                    <div className="md:col-span-2">
                                        <button
                                            type="button"
                                            onClick={handleTestSmtp}
                                            disabled={isTestingSmtp || !editSmtpHost}
                                            className="text-sm text-primary-600 hover:text-primary-700 font-medium"
                                        >
                                            {isTestingSmtp ? t('testingConnection') : t('testConnection')}
                                        </button>
                                    </div>
                                </div>
                            </div>

                            <div className="border-t border-gray-200 dark:border-gray-700 pt-6">
                                <h4 className="text-md font-medium text-gray-900 dark:text-white mb-4">{t('mfaRequired')}</h4>
                                <div className="space-y-4">
                                    <LockedToggle
                                        label={t('mfaRequired')}
                                        description={t('mfaRequiredDesc')}
                                        checked={editEnableTotp || isMfaLocked}
                                        onChange={(checked) => setEditEnableTotp(checked)}
                                        locked={isMfaLocked}
                                        reason={t('mfaComplianceReason')}
                                    />

                                    {/* SSO Auth Methods (read-only display) */}
                                    {company?.auth_methods && company.auth_methods.length > 0 && (
                                        <div className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                                            <p className="text-sm font-medium text-gray-900 dark:text-white mb-2">{t('enabledAuthMethods')}</p>
                                            <div className="flex flex-wrap gap-2">
                                                {company.auth_methods.includes('local') && (
                                                    <span className="px-2.5 py-1 rounded-full text-xs font-medium bg-gray-200 text-gray-700 dark:bg-gray-600 dark:text-gray-300">{t('authPassword')}</span>
                                                )}
                                                {company.auth_methods.includes('oidc') && (
                                                    <span className="px-2.5 py-1 rounded-full text-xs font-medium bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400">{t('authOidc')}</span>
                                                )}
                                                {company.auth_methods.includes('saml') && (
                                                    <span className="px-2.5 py-1 rounded-full text-xs font-medium bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400">{t('authSaml')}</span>
                                                )}
                                            </div>
                                            <p className="text-xs text-gray-500 dark:text-gray-400 mt-2">
                                                {t('ssoMethodsManagedHint')}
                                            </p>
                                        </div>
                                    )}
                                </div>
                            </div>

                            <div className="pt-4 flex justify-end">
                                <button
                                    onClick={handleSaveSettings}
                                    disabled={isSaving}
                                    className="flex items-center px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50"
                                >
                                    <Save className="w-4 h-4 mr-2" />
                                    {isSaving ? t('saving') : t('saveChanges')}
                                </button>
                            </div>
                        </div>

                        {/* Danger Zone - SuperAdmin only */}
                        {currentUser?.role === 'SuperAdmin' && company && tenant?.id !== company.id && (
                            <div className="bg-white dark:bg-gray-800 rounded-xl border-2 border-red-300 dark:border-red-800 shadow-sm overflow-hidden">
                                <div className="px-6 py-4 border-b border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/20">
                                    <h3 className="text-lg font-medium text-red-700 dark:text-red-400 flex items-center gap-2">
                                        <AlertTriangle className="w-5 h-5" />
                                        {t('dangerZone')}
                                    </h3>
                                    <p className="text-sm text-red-600 dark:text-red-400 mt-1">
                                        {t('dangerZoneDesc')}
                                    </p>
                                </div>
                                <div className="p-6 space-y-4">
                                    {/* Suspend/Unsuspend Company */}
                                    <div className="flex items-center justify-between p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                                        <div>
                                            <h4 className="text-sm font-medium text-gray-900 dark:text-white">
                                                {company.status === 'suspended' ? t('resumeCompany') : t('suspendCompany')}
                                            </h4>
                                            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                                                {company.status === 'suspended' 
                                                    ? t('resumeCompanyDesc')
                                                    : t('suspendCompanyDesc')}
                                            </p>
                                        </div>
                                        {company.status === 'suspended' ? (
                                            <button
                                                onClick={handleUnsuspendCompany}
                                                disabled={isSuspending}
                                                className="flex items-center px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 disabled:opacity-50 text-sm font-medium"
                                            >
                                                <Play className="w-4 h-4 mr-2" />
                                                {isSuspending ? tCommon('loading') || 'Processing...' : t('resumeCompany')}
                                            </button>
                                        ) : (
                                            <button
                                                onClick={() => setShowSuspendConfirm(true)}
                                                disabled={isSuspending}
                                                className="flex items-center px-4 py-2 bg-amber-600 text-white rounded-lg hover:bg-amber-700 disabled:opacity-50 text-sm font-medium"
                                            >
                                                <Ban className="w-4 h-4 mr-2" />
                                                {t('suspendCompany')}
                                            </button>
                                        )}
                                    </div>

                                    {/* Suspend Confirmation */}
                                    {showSuspendConfirm && (
                                        <div className="p-4 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg space-y-3">
                                            <p className="text-sm text-amber-800 dark:text-amber-200">
                                                {t('suspendConfirmTitle')} <strong>{company.name}</strong>? {t('suspendConfirmDesc')}
                                            </p>
                                            <div>
                                                <label className="block text-xs font-medium text-amber-700 dark:text-amber-300 mb-1">
                                                    {t('reasonOptional')}
                                                </label>
                                                <textarea
                                                    value={suspendReason}
                                                    onChange={(e) => setSuspendReason(e.target.value)}
                                                    placeholder={t('suspendReasonPlaceholder')}
                                                    rows={2}
                                                    className="w-full px-3 py-2 text-sm border border-amber-300 dark:border-amber-700 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white resize-none"
                                                />
                                            </div>
                                            <div className="flex gap-2">
                                                <button
                                                    onClick={handleSuspendCompany}
                                                    disabled={isSuspending}
                                                    className="flex-1 px-4 py-2 bg-amber-600 text-white rounded-lg text-sm font-medium hover:bg-amber-700 disabled:opacity-50"
                                                >
                                                    {isSuspending ? t('saving') : t('confirm')}
                                                </button>
                                                <button
                                                    onClick={() => { setShowSuspendConfirm(false); setSuspendReason(''); }}
                                                    className="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-200 rounded-lg text-sm font-medium hover:bg-gray-300 dark:hover:bg-gray-600"
                                                >
                                                    {t('cancel')}
                                                </button>
                                            </div>
                                        </div>
                                    )}

                                    {/* Delete Company */}
                                    <div className="flex items-center justify-between p-4 border border-red-200 dark:border-red-800 rounded-lg bg-red-50/50 dark:bg-red-900/10">
                                        <div>
                                            <h4 className="text-sm font-medium text-red-700 dark:text-red-400">
                                                {t('deleteCompany')}
                                            </h4>
                                            <p className="text-xs text-red-600 dark:text-red-400 mt-1">
                                                {t('deleteCompanyDesc')}
                                            </p>
                                        </div>
                                        <button
                                            onClick={() => setShowDeleteConfirm(true)}
                                            disabled={isDeleting}
                                            className="flex items-center px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50 text-sm font-medium"
                                        >
                                            <Trash2 className="w-4 h-4 mr-2" />
                                            {tCommon('delete') || 'Delete'}
                                        </button>
                                    </div>

                                    {/* Delete Confirmation */}
                                    {showDeleteConfirm && (
                                        <div className="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg space-y-3">
                                            <div className="flex items-start gap-2 text-sm text-red-800 dark:text-red-200">
                                                <AlertTriangle className="w-4 h-4 mt-0.5 flex-shrink-0" />
                                                <p>
                                                    <strong>{t('dangerZone')}:</strong> {t('deleteCompanyDesc')}
                                                </p>
                                            </div>
                                            <div>
                                                <label className="block text-xs font-medium text-red-700 dark:text-red-300 mb-1">
                                                    {t('deleteConfirmDesc')} <span className="font-mono bg-red-100 dark:bg-red-900/50 px-1 rounded">{company.name}</span>
                                                </label>
                                                <input
                                                    type="text"
                                                    value={deleteConfirmName}
                                                    onChange={(e) => setDeleteConfirmName(e.target.value)}
                                                    placeholder={company.name}
                                                    className="w-full px-3 py-2 text-sm border border-red-300 dark:border-red-700 rounded-md bg-white dark:bg-gray-800 text-gray-900 dark:text-white font-mono"
                                                />
                                            </div>
                                            <div className="flex gap-2">
                                                <button
                                                    onClick={handleDeleteCompany}
                                                    disabled={isDeleting || deleteConfirmName !== company.name}
                                                    className="flex-1 px-4 py-2 bg-red-600 text-white rounded-lg text-sm font-medium hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed"
                                                >
                                                    {isDeleting ? t('saving') : t('deleteCompany')}
                                                </button>
                                                <button
                                                    onClick={() => { setShowDeleteConfirm(false); setDeleteConfirmName(''); }}
                                                    className="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-200 rounded-lg text-sm font-medium hover:bg-gray-300 dark:hover:bg-gray-600"
                                                >
                                                    {t('cancel')}
                                                </button>
                                            </div>
                                        </div>
                                    )}
                                </div>
                            </div>
                        )}
                        </div>
                    </div>
                )}

                {activeTab === 'departments' && (
                    <div className="space-y-6">
                        <div className="flex justify-between items-center">
                            <h3 className="text-lg font-medium text-gray-900 dark:text-white">{t('departmentsTitle')}</h3>
                            <button
                                onClick={() => setIsAddingDept(true)}
                                className="flex items-center px-3 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 text-sm font-medium"
                            >
                                <Plus className="w-4 h-4 mr-2" />
                                {t('newDepartment')}
                            </button>
                        </div>

                        {isAddingDept && (
                            <div className="bg-gray-50 dark:bg-gray-800/50 p-4 rounded-lg border border-gray-200 dark:border-gray-700 flex items-end gap-4">
                                <div className="flex-1">
                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('colDepartmentName')}</label>
                                    <input
                                        type="text"
                                        value={newDeptName}
                                        onChange={(e) => setNewDeptName(e.target.value)}
                                        placeholder={t('departmentNamePlaceholder')}
                                        className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                    />
                                </div>
                                <button
                                    onClick={handleAddDepartment}
                                    disabled={!newDeptName.trim()}
                                    className="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 text-sm font-medium"
                                >
                                    {tCommon('add') || 'Add'}
                                </button>
                                <button
                                    onClick={() => {
                                        setIsAddingDept(false);
                                        setNewDeptName('');
                                    }}
                                    className="px-4 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-200 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-600 text-sm font-medium"
                                >
                                    {t('cancel')}
                                </button>
                            </div>
                        )}

                        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
                            <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                <thead className="bg-gray-50 dark:bg-gray-900/50">
                                    <tr>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">{t('colDepartmentName')}</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">{t('colDepartmentUsers')}</th>
                                        <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">{t('colDepartmentActions')}</th>
                                    </tr>
                                </thead>
                                <tbody className="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                                    {departments.length === 0 ? (
                                        <tr>
                                            <td colSpan={3} className="px-6 py-8 text-center text-gray-500 dark:text-gray-400">
                                                {t('noDepartments')}
                                            </td>
                                        </tr>
                                    ) : (
                                        departments.map((dept) => (
                                            <tr key={dept.id} className="hover:bg-gray-50 dark:hover:bg-gray-700/50">
                                                <td className="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900 dark:text-white">
                                                    {dept.name}
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                    {dept.user_count || 0}
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                                    <button className="text-red-600 hover:text-red-900 dark:hover:text-red-400">
                                                        <Trash2 className="w-4 h-4" />
                                                    </button>
                                                </td>
                                            </tr>
                                        ))
                                    )}
                                </tbody>
                            </table>
                        </div>
                    </div>
                )}

                {activeTab === 'users' && (
                    <div className="space-y-6">
                        <div className="flex justify-between items-center">
                            <h3 className="text-lg font-medium text-gray-900 dark:text-white">{t('usersTitle')}</h3>
                            <button
                                onClick={() => {
                                    setSelectedUser(null);
                                    setIsInviteModalOpen(true);
                                }}
                                className="flex items-center px-3 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 text-sm font-medium"
                            >
                                <Plus className="w-4 h-4 mr-2" />
                                {t('inviteUser')}
                            </button>
                        </div>

                        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
                            <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                                <thead className="bg-gray-50 dark:bg-gray-900/50">
                                    <tr>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">{t('colUser')}</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">{t('colRole')}</th>
                                        <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">{t('colStatus')}</th>
                                        <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">{t('colActions')}</th>
                                    </tr>
                                </thead>
                                <tbody className="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                                    {users.length === 0 ? (
                                        <tr>
                                            <td colSpan={4} className="px-6 py-8 text-center text-gray-500 dark:text-gray-400">
                                                {t('noUsers')}
                                            </td>
                                        </tr>
                                    ) : (
                                        users.map((user) => (
                                            <tr key={user.id} className="hover:bg-gray-50 dark:hover:bg-gray-700/50">
                                                <td className="px-6 py-4 whitespace-nowrap">
                                                    <div className="flex items-center">
                                                        <div className="h-8 w-8 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center text-primary-700 dark:text-primary-400 font-medium text-sm">
                                                            {user.name.charAt(0).toUpperCase()}
                                                        </div>
                                                        <div className="ml-3">
                                                            <div className="text-sm font-medium text-gray-900 dark:text-white">{user.name}</div>
                                                            <div className="text-xs text-gray-500 dark:text-gray-400">{user.email}</div>
                                                        </div>
                                                    </div>
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                                    <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-300">
                                                        {user.role}
                                                    </span>
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap">
                                                    <span className={clsx(
                                                        "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium",
                                                        user.status === 'active'
                                                            ? "bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-300"
                                                            : "bg-gray-100 dark:bg-gray-800 text-gray-800 dark:text-gray-300"
                                                    )}>
                                                        {user.status === 'active' ? t('statusActive') : t('statusSuspended')}
                                                    </span>
                                                </td>
                                                <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                                    <button
                                                        onClick={() => handleEditUser(user)}
                                                        className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                                                    >
                                                        <Settings className="w-4 h-4" />
                                                    </button>
                                                </td>
                                            </tr>
                                        ))
                                    )}
                                </tbody>
                            </table>
                        </div>
                    </div>
                )}

                {activeTab === 'notifications' && (
                    <div className="max-w-6xl mx-auto space-y-6">
                        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
                            <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                                <h3 className="text-lg font-medium text-gray-900 dark:text-white flex items-center gap-2">
                                    <Bell className="w-5 h-5 text-primary-600" />
                                    {t('notificationsTitle')}
                                </h3>
                                <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                                    {t('notificationsDesc')}
                                </p>
                            </div>
                            
                            {/* Role Tabs */}
                            <div className="px-6 py-3 bg-gray-50 dark:bg-gray-800/50 border-b border-gray-200 dark:border-gray-700">
                                <div className="flex flex-wrap gap-2">
                                    <button
                                        onClick={() => setSelectedNotificationRole(null)}
                                        className={clsx(
                                            "px-4 py-2 rounded-lg text-sm font-medium transition-colors",
                                            selectedNotificationRole === null
                                                ? "bg-primary-600 text-white"
                                                : "bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 border border-gray-200 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-600"
                                        )}
                                    >
                                        {t('roleAllUsers')}
                                    </button>
                                    {availableRoles.map((role) => (
                                        <button
                                            key={role}
                                            onClick={() => setSelectedNotificationRole(role)}
                                            className={clsx(
                                                "px-4 py-2 rounded-lg text-sm font-medium transition-colors",
                                                selectedNotificationRole === role
                                                    ? "bg-primary-600 text-white"
                                                    : "bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 border border-gray-200 dark:border-gray-600 hover:bg-gray-100 dark:hover:bg-gray-600"
                                            )}
                                        >
                                            {role}
                                            {notificationsByRole[role]?.length > 0 && (
                                                <span className="ml-2 text-xs bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300 px-1.5 py-0.5 rounded">
                                                    {t('notifCustom')}
                                                </span>
                                            )}
                                        </button>
                                    ))}
                                </div>
                                {selectedNotificationRole && (
                                    <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">
                                        {t('notifOverrideDesc', { role: selectedNotificationRole })}
                                    </p>
                                )}
                            </div>
                            
                            {isLoadingNotifications ? (
                                <div className="p-6 space-y-4">
                                    {[...Array(4)].map((_, i) => (
                                        <div key={i} className="animate-pulse flex items-center justify-between">
                                            <div className="space-y-2">
                                                <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-32"></div>
                                                <div className="h-3 bg-gray-200 dark:bg-gray-700 rounded w-48"></div>
                                            </div>
                                            <div className="flex space-x-4">
                                                <div className="h-6 w-12 bg-gray-200 dark:bg-gray-700 rounded"></div>
                                            </div>
                                        </div>
                                    ))}
                                </div>
                            ) : (
                                <div className="divide-y divide-gray-200 dark:divide-gray-700">
                                    {notificationSettings.map((setting) => {
                                        const labelInfo = notificationLabels[setting.event_type] || { 
                                            label: setting.event_type, 
                                            description: '' 
                                        };
                                        const isInherited = setting.inherited === true;
                                        return (
                                            <div key={setting.event_type} className={clsx(
                                                "p-6",
                                                isInherited && "bg-gray-50/50 dark:bg-gray-800/30"
                                            )}>
                                                <div className="flex items-start justify-between">
                                                    <div className="flex-1">
                                                        <div className="flex items-center gap-2">
                                                            <h4 className="text-sm font-medium text-gray-900 dark:text-white">
                                                                {labelInfo.label}
                                                            </h4>
                                                            {isInherited && (
                                                                <span className="text-xs bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400 px-2 py-0.5 rounded">
                                                                    {t('inherited')}
                                                                </span>
                                                            )}
                                                        </div>
                                                        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                                                            {labelInfo.description}
                                                        </p>
                                                    </div>
                                                    <div className="ml-4">
                                                        <label className="flex items-center space-x-2 cursor-pointer">
                                                            <input
                                                                type="checkbox"
                                                                checked={setting.enabled}
                                                                onChange={(e) => handleSaveNotificationSetting(setting.event_type, 'enabled', e.target.checked)}
                                                                disabled={isSavingNotifications}
                                                                className="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                                                            />
                                                            <span className="text-sm text-gray-600 dark:text-gray-300">{t('enabled')}</span>
                                                        </label>
                                                    </div>
                                                </div>
                                                
                                                {setting.enabled && (
                                                    <div className="mt-4 pt-4 border-t border-gray-100 dark:border-gray-700">
                                                        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                                                            {/* Enforce Email */}
                                                            <div className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                                                                <div className="flex items-center space-x-2">
                                                                    <Mail className="w-4 h-4 text-gray-400" />
                                                                    <div>
                                                                        <span className="text-sm font-medium text-gray-700 dark:text-gray-300">{t('forceEmail')}</span>
                                                                        <p className="text-xs text-gray-500 dark:text-gray-400">{t('usersCannotDisable')}</p>
                                                                    </div>
                                                                </div>
                                                                <input
                                                                    type="checkbox"
                                                                    checked={setting.email_enforced}
                                                                    onChange={(e) => handleSaveNotificationSetting(setting.event_type, 'email_enforced', e.target.checked)}
                                                                    disabled={isSavingNotifications}
                                                                    className="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                                                                />
                                                            </div>
                                                            
                                                            {/* Enforce In-App */}
                                                            <div className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                                                                <div className="flex items-center space-x-2">
                                                                    <BellRing className="w-4 h-4 text-gray-400" />
                                                                    <div>
                                                                        <span className="text-sm font-medium text-gray-700 dark:text-gray-300">{t('forceInApp')}</span>
                                                                        <p className="text-xs text-gray-500 dark:text-gray-400">{t('usersCannotDisable')}</p>
                                                                    </div>
                                                                </div>
                                                                <input
                                                                    type="checkbox"
                                                                    checked={setting.in_app_enforced}
                                                                    onChange={(e) => handleSaveNotificationSetting(setting.event_type, 'in_app_enforced', e.target.checked)}
                                                                    disabled={isSavingNotifications}
                                                                    className="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                                                                />
                                                            </div>
                                                            
                                                            {/* Default Email */}
                                                            <div className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                                                                <div className="flex items-center space-x-2">
                                                                    <Mail className="w-4 h-4 text-gray-400" />
                                                                    <div>
                                                                        <span className="text-sm font-medium text-gray-700 dark:text-gray-300">{t('defaultEmail')}</span>
                                                                        <p className="text-xs text-gray-500 dark:text-gray-400">{t('forNewUsers')}</p>
                                                                    </div>
                                                                </div>
                                                                <input
                                                                    type="checkbox"
                                                                    checked={setting.default_email}
                                                                    onChange={(e) => handleSaveNotificationSetting(setting.event_type, 'default_email', e.target.checked)}
                                                                    disabled={isSavingNotifications || setting.email_enforced}
                                                                    className="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500 disabled:opacity-50"
                                                                />
                                                            </div>
                                                            
                                                            {/* Default In-App */}
                                                            <div className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                                                                <div className="flex items-center space-x-2">
                                                                    <BellRing className="w-4 h-4 text-gray-400" />
                                                                    <div>
                                                                        <span className="text-sm font-medium text-gray-700 dark:text-gray-300">{t('defaultInApp')}</span>
                                                                        <p className="text-xs text-gray-500 dark:text-gray-400">{t('forNewUsers')}</p>
                                                                    </div>
                                                                </div>
                                                                <input
                                                                    type="checkbox"
                                                                    checked={setting.default_in_app}
                                                                    onChange={(e) => handleSaveNotificationSetting(setting.event_type, 'default_in_app', e.target.checked)}
                                                                    disabled={isSavingNotifications || setting.in_app_enforced}
                                                                    className="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500 disabled:opacity-50"
                                                                />
                                                            </div>
                                                        </div>
                                                    </div>
                                                )}
                                            </div>
                                        );
                                    })}
                                </div>
                            )}
                        </div>
                        
                        <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
                            <div className="flex">
                                <Info className="w-5 h-5 text-blue-500 mr-3 flex-shrink-0 mt-0.5" />
                                <div>
                                    <h4 className="text-sm font-medium text-blue-800 dark:text-blue-300">{t('howItWorks')}</h4>
                                    <p className="text-sm text-blue-700 dark:text-blue-400 mt-1">
                                        <strong>{t('roleBased')}</strong> {t('roleBasedDesc')}<br/>
                                        <strong>{t('superAdmins')}</strong> {t('superAdminsDesc')}<br/>
                                        <strong>{t('forceEmailInApp')}</strong> {t('forceEmailInAppDesc')}<br/>
                                        <strong>{t('inheritedLabel')}</strong> {t('inheritedLabelDesc')}
                                    </p>
                                </div>
                            </div>
                        </div>
                    </div>
                )}

                {activeTab === 'email-templates' && company && (
                    <TenantEmailTemplates
                        tenantId={company.id}
                        authFetch={authFetch}
                    />
                )}

                {activeTab === 'ai' && company && (
                    <TenantAiSettings
                        tenantId={company.id}
                        authFetch={authFetch}
                    />
                )}


                {activeTab === 'document-workflow' && company && (
                    <div className="max-w-6xl mx-auto space-y-6">
                        {/* Enable/Disable Card */}
                        <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                            <div className="flex items-center justify-between">
                                <div className="flex-1">
                                    <h3 className="text-lg font-semibold text-gray-900 dark:text-white">{t('docApprovalWorkflow')}</h3>
                                    <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                                        {t('docApprovalWorkflowDesc')}
                                    </p>
                                </div>
                                <button
                                    type="button"
                                    onClick={async () => {
                                        const newVal = !editApprovalWorkflow;
                                        setEditApprovalWorkflow(newVal);
                                        try {
                                            const res = await authFetch(`/api/tenants/${company.id}/edit`, {
                                                method: 'PUT',
                                                headers: { 'Content-Type': 'application/json' },
                                                body: JSON.stringify({ approval_workflow_enabled: newVal }),
                                            });
                                            if (res.ok) {
                                                setCompany(prev => prev ? { ...prev, approval_workflow_enabled: newVal } : prev);
                                                await refreshUser();
                                            } else {
                                                setEditApprovalWorkflow(!newVal);
                                            }
                                        } catch (e) {
                                            setEditApprovalWorkflow(!newVal);
                                        }
                                    }}
                                    className={clsx(
                                        'relative inline-flex h-6 w-11 flex-shrink-0 items-center rounded-full transition-colors ml-6',
                                        editApprovalWorkflow ? 'bg-primary-600' : 'bg-gray-300 dark:bg-gray-600'
                                    )}
                                >
                                    <span className={clsx(
                                        'inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform',
                                        editApprovalWorkflow ? 'translate-x-6' : 'translate-x-1'
                                    )} />
                                </button>
                            </div>
                            {editApprovalWorkflow && (
                                <div className="mt-4 p-3 bg-green-50 dark:bg-green-900/20 rounded-lg border border-green-200 dark:border-green-800">
                                    <p className="text-sm text-green-800 dark:text-green-300">
                                        {t('workflowActiveBanner')}
                                    </p>
                                </div>
                            )}
                        </div>

                        {/* Policies Card */}
                        {editApprovalWorkflow && (
                            <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700">
                                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
                                    <div>
                                        <h3 className="text-lg font-semibold text-gray-900 dark:text-white">{t('approvalPolicies')}</h3>
                                        <p className="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
                                            {t('approvalPoliciesDesc')}
                                        </p>
                                    </div>
                                    {!showAddPolicy && (
                                        <button
                                            onClick={() => setShowAddPolicy(true)}
                                            className="px-4 py-2 text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 rounded-lg transition-colors flex-shrink-0 ml-4"
                                        >
                                            {t('addPolicy')}
                                        </button>
                                    )}
                                </div>

                                <div className="p-6">
                                    {/* Add/Edit Policy Form */}
                                    {showAddPolicy && (
                                        <div className="mb-6 p-5 bg-gray-50 dark:bg-gray-700/50 rounded-xl border border-gray-200 dark:border-gray-600">
                                            <h4 className="text-sm font-semibold text-gray-900 dark:text-white mb-4">
                                                {editingPolicyId ? t('editPolicy') : t('newApprovalPolicy')}
                                            </h4>
                                            <div className="space-y-4">
                                                <div>
                                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('policyName')}</label>
                                                    <input
                                                        type="text"
                                                        placeholder={t('policyNamePlaceholder')}
                                                        value={newPolicyName}
                                                        onChange={(e) => setNewPolicyName(e.target.value)}
                                                        className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                                                    />
                                                </div>
                                                <div>
                                                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('scope')}</label>
                                                    <select
                                                        value={newPolicyScope}
                                                        onChange={(e) => setNewPolicyScope(e.target.value)}
                                                        className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                                                    >
                                                        <option value="all">{t('scopeAllUploads')}</option>
                                                        <option value="department">{t('scopeDepartment')}</option>
                                                        <option value="company_folder">{t('scopeCompanyFolders')}</option>
                                                        <option value="file_type">{t('scopeFileType')}</option>
                                                        <option value="file_size">{t('scopeFileSize')}</option>
                                                        <option value="role">{t('scopeUserRole')}</option>
                                                        <option value="private_files">{t('scopePrivateFiles')}</option>
                                                    </select>
                                                </div>
                                                {newPolicyScope === 'department' && (
                                                    <div>
                                                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('departmentLabel')}</label>
                                                        <select
                                                            value={newPolicyScopeValue}
                                                            onChange={(e) => setNewPolicyScopeValue(e.target.value)}
                                                            className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                                                        >
                                                            <option value="">{t('selectDepartment')}</option>
                                                            {departments.map(dept => (
                                                                <option key={dept.id} value={dept.id}>{dept.name}</option>
                                                            ))}
                                                        </select>
                                                    </div>
                                                )}
                                                {newPolicyScope === 'file_type' && (
                                                    <div>
                                                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('fileExtensions')}</label>
                                                        <input
                                                            type="text"
                                                            placeholder={t('fileExtensionsPlaceholder')}
                                                            value={newPolicyScopeValue}
                                                            onChange={(e) => setNewPolicyScopeValue(e.target.value)}
                                                            className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                                                        />
                                                        <p className="text-xs text-gray-400 mt-1">{t('fileExtensionsHint')}</p>
                                                    </div>
                                                )}
                                                {newPolicyScope === 'file_size' && (
                                                    <div>
                                                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('sizeThresholdMb')}</label>
                                                        <input
                                                            type="number"
                                                            min="1"
                                                            placeholder="10"
                                                            value={newPolicyScopeValue ? String(Math.round(Number(newPolicyScopeValue) / 1048576)) : ''}
                                                            onChange={(e) => setNewPolicyScopeValue(String(Number(e.target.value) * 1048576))}
                                                            className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                                                        />
                                                        <p className="text-xs text-gray-400 mt-1">{t('sizeThresholdHint')}</p>
                                                    </div>
                                                )}
                                                {newPolicyScope === 'role' && (
                                                    <div>
                                                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">{t('userRole')}</label>
                                                        <select
                                                            value={newPolicyScopeValue}
                                                            onChange={(e) => setNewPolicyScopeValue(e.target.value)}
                                                            className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                                                        >
                                                            <option value="">{t('selectRole')}</option>
                                                            <option value="Employee">Employee</option>
                                                            <option value="Manager">Manager</option>
                                                        </select>
                                                        <p className="text-xs text-gray-400 mt-1">{t('userRoleHint')}</p>
                                                    </div>
                                                )}
                                            </div>
                                            <div className="flex justify-end space-x-3 mt-5 pt-4 border-t border-gray-200 dark:border-gray-600">
                                                <button
                                                    onClick={() => { setShowAddPolicy(false); setEditingPolicyId(null); setNewPolicyName(''); setNewPolicyScope('all'); setNewPolicyScopeValue(''); }}
                                                    className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600 rounded-lg transition-colors"
                                                >
                                                    {t('cancel')}
                                                </button>
                                                <button
                                                    onClick={async () => {
                                                        if (!newPolicyName.trim()) return;
                                                        try {
                                                            if (editingPolicyId) {
                                                                const res = await authFetch(`/api/approvals/${company.id}/policies/${editingPolicyId}`, {
                                                                    method: 'PUT',
                                                                    headers: { 'Content-Type': 'application/json' },
                                                                    body: JSON.stringify({
                                                                        name: newPolicyName,
                                                                        scope: newPolicyScope,
                                                                        scope_value: newPolicyScope === 'department' ? newPolicyScopeValue : null,
                                                                    }),
                                                                });
                                                                if (res.ok) {
                                                                    const data = await res.json();
                                                                    setApprovalPolicies(prev => prev.map(p => p.id === editingPolicyId ? data.policy : p));
                                                                }
                                                            } else {
                                                                const res = await authFetch(`/api/approvals/${company.id}/policies`, {
                                                                    method: 'POST',
                                                                    headers: { 'Content-Type': 'application/json' },
                                                                    body: JSON.stringify({
                                                                        name: newPolicyName,
                                                                        scope: newPolicyScope,
                                                                        scope_value: newPolicyScope === 'department' ? newPolicyScopeValue : null,
                                                                    }),
                                                                });
                                                                if (res.ok) {
                                                                    const data = await res.json();
                                                                    setApprovalPolicies(prev => [...prev, data.policy]);
                                                                }
                                                            }
                                                            setShowAddPolicy(false);
                                                            setEditingPolicyId(null);
                                                            setNewPolicyName('');
                                                            setNewPolicyScope('all');
                                                            setNewPolicyScopeValue('');
                                                        } catch (e) { console.error(e); }
                                                    }}
                                                    disabled={!newPolicyName.trim() || (['department', 'file_type', 'file_size', 'role'].includes(newPolicyScope) && !newPolicyScopeValue)}
                                                    className="px-4 py-2 text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg transition-colors"
                                                >
                                                    {editingPolicyId ? t('saveChanges') : t('createPolicy')}
                                                </button>
                                            </div>
                                        </div>
                                    )}

                                    {/* Policy List */}
                                    {approvalPolicies.length === 0 && !showAddPolicy ? (
                                        <div className="text-center py-8">
                                            <div className="w-12 h-12 mx-auto mb-3 rounded-full bg-gray-100 dark:bg-gray-700 flex items-center justify-center">
                                                <Shield className="w-6 h-6 text-gray-400" />
                                            </div>
                                            <p className="text-sm font-medium text-gray-900 dark:text-white">{t('noPolicies')}</p>
                                            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">{t('noPoliciesHint')}</p>
                                        </div>
                                    ) : (
                                        <div className="space-y-3">
                                            {approvalPolicies.map((policy) => (
                                                <div key={policy.id} className={clsx(
                                                    "rounded-lg border p-4 transition-colors",
                                                    policy.is_active
                                                        ? "bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700"
                                                        : "bg-gray-50 dark:bg-gray-800/50 border-gray-200 dark:border-gray-700 opacity-60"
                                                )}>
                                                    <div className="flex items-start justify-between">
                                                        <div className="flex-1 min-w-0">
                                                            <div className="flex items-center space-x-2">
                                                                <p className="text-sm font-semibold text-gray-900 dark:text-white">{policy.name}</p>
                                                                <span className={clsx(
                                                                    "px-2 py-0.5 text-[10px] font-medium rounded-full",
                                                                    policy.is_active
                                                                        ? "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400"
                                                                        : "bg-gray-100 text-gray-500 dark:bg-gray-700 dark:text-gray-400"
                                                                )}>
                                                                    {policy.is_active ? t('active') : t('inactive')}
                                                                </span>
                                                            </div>
                                                            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                                                                {policy.scope === 'all' && t('policyAppliesAll')}
                                                                {policy.scope === 'company_folder' && t('policyAppliesCompanyFolders')}
                                                                {policy.scope === 'department' && t('policyAppliesDept')}
                                                                {policy.scope === 'file_type' && t('policyAppliesExt', { ext: policy.scope_value || '' })}
                                                                {policy.scope === 'file_size' && t('policyAppliesSize', { size: policy.scope_value ? Math.round(Number(policy.scope_value) / 1048576) : '?' })}
                                                                {policy.scope === 'role' && t('policyAppliesRole', { role: policy.scope_value || '' })}
                                                                {policy.scope === 'private_files' && t('policyAppliesPrivate')}
                                                            </p>
                                                        </div>
                                                        <div className="flex items-center space-x-2 ml-4">
                                                            <button
                                                                type="button"
                                                                onClick={async () => {
                                                                    try {
                                                                        await authFetch(`/api/approvals/${company.id}/policies/${policy.id}`, {
                                                                            method: 'PUT',
                                                                            headers: { 'Content-Type': 'application/json' },
                                                                            body: JSON.stringify({ is_active: !policy.is_active }),
                                                                        });
                                                                        setApprovalPolicies(prev => prev.map(p => p.id === policy.id ? { ...p, is_active: !p.is_active } : p));
                                                                    } catch (e) { console.error(e); }
                                                                }}
                                                                className={clsx(
                                                                    'relative inline-flex h-6 w-11 items-center rounded-full transition-colors',
                                                                    policy.is_active ? 'bg-primary-600' : 'bg-gray-300 dark:bg-gray-600'
                                                                )}
                                                            >
                                                                <span className={clsx(
                                                                    'inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform',
                                                                    policy.is_active ? 'translate-x-6' : 'translate-x-1'
                                                                )} />
                                                            </button>
                                                            <button
                                                                onClick={() => {
                                                                    setEditingPolicyId(policy.id);
                                                                    setNewPolicyName(policy.name);
                                                                    setNewPolicyScope(policy.scope);
                                                                    setNewPolicyScopeValue(policy.scope_value || '');
                                                                    setShowAddPolicy(true);
                                                                }}
                                                                className="p-1.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded transition-colors"
                                                                title={t('editPolicy')}
                                                            >
                                                                <Edit2 className="w-4 h-4" />
                                                            </button>
                                                            <button
                                                                onClick={async () => {
                                                                    const ok = await modalConfirm({
                                                                        title: tCommon('deleteConfirmTitle') || 'Confirm Delete',
                                                                        description: t('deletePolicyConfirm', { name: policy.name }),
                                                                        variant: 'destructive'
                                                                    });
                                                                    if (!ok) return;
                                                                    try {
                                                                        await authFetch(`/api/approvals/${company.id}/policies/${policy.id}`, { method: 'DELETE' });
                                                                        setApprovalPolicies(prev => prev.filter(p => p.id !== policy.id));
                                                                    } catch (e) { console.error(e); }
                                                                }}
                                                                className="p-1.5 text-gray-400 hover:text-red-500 dark:hover:text-red-400 rounded transition-colors"
                                                                title={t('deletePolicy')}
                                                            >
                                                                <Trash2 className="w-4 h-4" />
                                                            </button>
                                                        </div>
                                                    </div>
                                                </div>
                                            ))}
                                        </div>
                                    )}
                                </div>
                            </div>
                        )}

                        {/* How It Works Card */}
                        {editApprovalWorkflow && (
                            <div className="bg-white dark:bg-gray-800 rounded-xl shadow-sm border border-gray-200 dark:border-gray-700 p-6">
                                <h3 className="text-sm font-semibold text-gray-900 dark:text-white mb-3">{t('howItWorks')}</h3>
                                <div className="space-y-3 text-xs text-gray-600 dark:text-gray-400">
                                    <div className="flex items-start space-x-3">
                                        <span className="flex-shrink-0 w-5 h-5 rounded-full bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 flex items-center justify-center text-[10px] font-bold">1</span>
                                        <p>{t('workflowStep1')}</p>
                                    </div>
                                    <div className="flex items-start space-x-3">
                                        <span className="flex-shrink-0 w-5 h-5 rounded-full bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 flex items-center justify-center text-[10px] font-bold">2</span>
                                        <p>{t('workflowStep2')}</p>
                                    </div>
                                    <div className="flex items-start space-x-3">
                                        <span className="flex-shrink-0 w-5 h-5 rounded-full bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 flex items-center justify-center text-[10px] font-bold">3</span>
                                        <p>{t('workflowStep3')}</p>
                                    </div>
                                    <div className="flex items-start space-x-3">
                                        <span className="flex-shrink-0 w-5 h-5 rounded-full bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 flex items-center justify-center text-[10px] font-bold">4</span>
                                        <p>{t('workflowStep4')}</p>
                                    </div>
                                </div>
                            </div>
                        )}
                    </div>
                )}

                {activeTab === 'backup' && company && (
                    <div className="space-y-6">
                        {/* Backup Enable/Disable Toggle */}
                        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm p-6">
                            <div className="flex items-center justify-between">
                                <div>
                                    <h3 className="text-sm font-medium text-gray-900 dark:text-white">{t('tabBackup')}</h3>
                                    <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                                        {t('backupDesc')}
                                    </p>
                                </div>
                                <button
                                    onClick={async () => {
                                        const newVal = !(company.backup_enabled !== false);
                                        try {
                                            const endpoint = currentUser?.role === 'SuperAdmin'
                                                ? `/api/tenants/${company.id}`
                                                : `/api/tenants/${company.id}/edit`;
                                            const method = 'PUT';
                                            await authFetch(endpoint, {
                                                method,
                                                headers: { 'Content-Type': 'application/json' },
                                                body: JSON.stringify({ backup_enabled: newVal }),
                                            });
                                            setCompany({ ...company, backup_enabled: newVal });
                                        } catch (err) {
                                            console.error('Failed to toggle backup:', err);
                                        }
                                    }}
                                    className={clsx(
                                        "relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out",
                                        company.backup_enabled !== false ? "bg-primary-600" : "bg-gray-200 dark:bg-gray-600"
                                    )}
                                >
                                    <span className={clsx(
                                        "pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out",
                                        company.backup_enabled !== false ? "translate-x-5" : "translate-x-0"
                                    )} />
                                </button>
                            </div>
                        </div>

                        {company.backup_enabled !== false && (
                            <BackupRestore type="tenant" tenantId={company.id} />
                        )}
                    </div>
                )}

                {activeTab === 'audit' && (
                    <div className="max-w-6xl mx-auto space-y-6">
                        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
                            <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                                <h3 className="text-lg font-medium text-gray-900 dark:text-white flex items-center gap-2">
                                    <Activity className="w-5 h-5 text-primary-600" />
                                    {t('auditTitle')}
                                </h3>
                                <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                                    {t('auditDesc')}
                                </p>
                            </div>
                            <div className="p-6 space-y-4">
                                <LockedToggle
                                    label={t('auditLogLogins')}
                                    description={t('auditLogLoginsDesc')}
                                    checked={auditLogLogins || auditSettingsLocked}
                                    onChange={setAuditLogLogins}
                                    locked={auditSettingsLocked}
                                    reason={t('complianceLockedAudit')}
                                />

                                <LockedToggle
                                    label={t('auditLogFiles')}
                                    description={t('auditLogFilesDesc')}
                                    checked={auditLogFileOperations || auditSettingsLocked}
                                    onChange={setAuditLogFileOperations}
                                    locked={auditSettingsLocked}
                                    reason={t('complianceLockedAudit')}
                                />

                                <LockedToggle
                                    label={t('auditLogUsers')}
                                    description={t('auditLogUsersDesc')}
                                    checked={auditLogUserChanges || auditSettingsLocked}
                                    onChange={setAuditLogUserChanges}
                                    locked={auditSettingsLocked}
                                    reason={t('complianceLockedAudit')}
                                />

                                <LockedToggle
                                    label={t('auditLogSettings')}
                                    description={t('auditLogSettingsDesc')}
                                    checked={auditLogSettingsChanges || auditSettingsLocked}
                                    onChange={setAuditLogSettingsChanges}
                                    locked={auditSettingsLocked}
                                    reason={t('complianceLockedAudit')}
                                />

                                <LockedToggle
                                    label={t('auditLogRoles')}
                                    description={t('auditLogRolesDesc')}
                                    checked={auditLogRoleChanges || auditSettingsLocked}
                                    onChange={setAuditLogRoleChanges}
                                    locked={auditSettingsLocked}
                                    reason={t('complianceLockedAudit')}
                                />
                            </div>
                        </div>

                        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
                            <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                                <h3 className="text-lg font-medium text-gray-900 dark:text-white">{t('logRetention')}</h3>
                                <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                                    {t('logRetentionDesc')}
                                </p>
                            </div>
                            <div className="p-6">
                                <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
                                    {[30, 60, 90, 180, 365, 730].map((days) => (
                                        <div
                                            key={days}
                                            onClick={() => setAuditRetentionDays(days)}
                                            className={clsx(
                                                "relative rounded-lg border p-4 cursor-pointer flex flex-col items-center justify-center hover:border-primary-300 dark:hover:border-primary-500 transition-colors",
                                                auditRetentionDays === days
                                                    ? "bg-primary-50 dark:bg-primary-900/20 border-primary-500 ring-1 ring-primary-500"
                                                    : "border-gray-300 dark:border-gray-600"
                                            )}
                                        >
                                            <span className="text-xl font-bold text-gray-900 dark:text-white">{days}</span>
                                            <span className="text-xs text-gray-500 dark:text-gray-400">{t('days')}</span>
                                            {auditRetentionDays === days && (
                                                <div className="absolute top-1 right-1">
                                                    <CheckCircle className="h-4 w-4 text-primary-600 dark:text-primary-400" />
                                                </div>
                                            )}
                                        </div>
                                    ))}
                                </div>
                            </div>
                        </div>

                        <div className="flex justify-end">
                            <button
                                onClick={handleSaveAuditSettings}
                                disabled={isSavingAudit}
                                className="flex items-center px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50"
                            >
                                <Save className="w-4 h-4 mr-2" />
                                {isSavingAudit ? t('savingAuditSettings') : t('saveAuditSettings')}
                            </button>
                        </div>

                        <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
                            <p className="text-sm text-blue-800 dark:text-blue-200">
                                {t('auditLogsPageHint')}
                            </p>
                        </div>
                    </div>
                )}
            </div>

            <InviteUserModal
                isOpen={isInviteModalOpen}
                onClose={() => {
                    setIsInviteModalOpen(false);
                    setSelectedUser(null);
                }}
                onSubmit={handleUserSubmit}
                targetTenantId={company.id}
                initialData={selectedUser ? {
                    name: selectedUser.name,
                    email: selectedUser.email,
                    role: selectedUser.role,
                    department_id: selectedUser.department_id || '',
                    allowed_department_ids: selectedUser.allowed_department_ids || [],
                    password: '', // Password not editable here
                    allowed_tenant_ids: selectedUser.allowed_tenant_ids || [],
                } : undefined}
            />
        </div>
    );
}
