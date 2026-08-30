import { createContext, useContext, useState, ReactNode, useEffect, useCallback } from 'react';
import { useAuth, useAuthFetch } from './AuthContext';

export type ComplianceMode = 'Standard' | 'HIPAA' | 'SOX' | 'GDPR' | 'None';

export interface EnforcedSetting {
    name: string;
    description: string;
    locked: boolean;
    forced_value?: any;
}

export interface ComplianceRestrictions {
    mode: string;
    mode_label: string;
    is_active: boolean;
    mfa_required: boolean;
    mfa_locked: boolean;
    session_timeout_minutes: number | null;
    session_timeout_locked: boolean;
    audit_logging_mandatory: boolean;
    audit_settings_locked: boolean;
    public_sharing_blocked: boolean;
    public_sharing_locked: boolean;
    file_versioning_required: boolean;
    retention_policy_locked: boolean;
    min_retention_days: number | null;
    deletion_requests_allowed: boolean;
    consent_tracking_required: boolean;
    export_logging_required: boolean;
    enforced_settings: EnforcedSetting[];
}

interface SettingsContextType {
    complianceMode: ComplianceMode;
    setComplianceMode: (mode: ComplianceMode) => void;
    retentionPolicyDays: number;
    setRetentionPolicyDays: (days: number) => void;
    encryptionStandard: string;
    restrictions: ComplianceRestrictions | null;
    isComplianceActive: boolean;
    isLoading: boolean;
    refreshRestrictions: () => Promise<void>;
    canModifySetting: (setting: string) => boolean;
}

const defaultRestrictions: ComplianceRestrictions = {
    mode: 'Standard',
    mode_label: 'Standard',
    is_active: false,
    mfa_required: false,
    mfa_locked: false,
    session_timeout_minutes: null,
    session_timeout_locked: false,
    audit_logging_mandatory: false,
    audit_settings_locked: false,
    public_sharing_blocked: false,
    public_sharing_locked: false,
    file_versioning_required: false,
    retention_policy_locked: false,
    min_retention_days: null,
    deletion_requests_allowed: true,
    consent_tracking_required: false,
    export_logging_required: false,
    enforced_settings: [],
};

const SettingsContext = createContext<SettingsContextType | undefined>(undefined);

export function SettingsProvider({ children }: { children: ReactNode }) {
    const [complianceMode, setComplianceMode] = useState<ComplianceMode>('Standard');
    const [retentionPolicyDays, setRetentionPolicyDays] = useState<number>(30);
    const [restrictions, setRestrictions] = useState<ComplianceRestrictions | null>(null);
    const [isLoading, setIsLoading] = useState(false);
    const { tenant, isAuthenticated } = useAuth();
    const authFetch = useAuthFetch();
    const encryptionStandard = 'AES-256';

    const refreshRestrictions = useCallback(async () => {
        if (!isAuthenticated) return;
        
        setIsLoading(true);
        try {
            const response = await authFetch('/api/compliance/restrictions');
            if (response.ok) {
                const data = await response.json();
                setRestrictions(data);
            }
        } catch (error) {
            console.error('Failed to fetch compliance restrictions:', error);
        } finally {
            setIsLoading(false);
        }
    }, [authFetch, isAuthenticated]);

    useEffect(() => {
        if (tenant) {
            // Map tenant compliance mode string to ComplianceMode type
            let mode: ComplianceMode = 'Standard';
            const tenantMode = tenant.compliance_mode?.toUpperCase();
            if (tenantMode === 'HIPAA') mode = 'HIPAA';
            else if (tenantMode === 'SOX' || tenantMode === 'SOC2') mode = 'SOX';
            else if (tenantMode === 'GDPR') mode = 'GDPR';
            else if (tenantMode === 'NONE' || !tenantMode) mode = 'Standard';
            
            setComplianceMode(mode);
            setRetentionPolicyDays(tenant.retention_policy_days || 30);
            
            // Fetch restrictions when tenant changes
            refreshRestrictions();
        }
    }, [tenant, refreshRestrictions]);

    const isComplianceActive = restrictions?.is_active || 
        (complianceMode !== 'Standard' && complianceMode !== 'None');

    const canModifySetting = useCallback((setting: string): boolean => {
        if (!restrictions) return true;
        
        switch (setting) {
            case 'mfa_required':
            case 'enable_totp':
                return !restrictions.mfa_locked;
            case 'session_timeout_minutes':
                return !restrictions.session_timeout_locked;
            case 'public_sharing_enabled':
                return !restrictions.public_sharing_locked;
            case 'log_logins':
            case 'log_file_operations':
            case 'log_user_changes':
            case 'log_settings_changes':
            case 'log_role_changes':
                return !restrictions.audit_settings_locked;
            case 'retention_policy_days':
                return !restrictions.retention_policy_locked;
            default:
                return true;
        }
    }, [restrictions]);

    return (
        <SettingsContext.Provider value={{
            complianceMode,
            setComplianceMode,
            retentionPolicyDays,
            setRetentionPolicyDays,
            encryptionStandard,
            restrictions,
            isComplianceActive,
            isLoading,
            refreshRestrictions,
            canModifySetting,
        }}>
            {children}
        </SettingsContext.Provider>
    );
}

export function useSettings() {
    const context = useContext(SettingsContext);
    if (context === undefined) {
        throw new Error('useSettings must be used within a SettingsProvider');
    }
    return context;
}

// Helper function to get the display label for a compliance mode
export function getComplianceModeLabel(mode: string): string {
    switch (mode?.toUpperCase()) {
        case 'HIPAA':
            return 'HIPAA Secure';
        case 'SOX':
        case 'SOC2':
            return 'SOX Governed';
        case 'GDPR':
            return 'GDPR Active';
        default:
            return 'Standard';
    }
}

// Helper function to get enforcement summary for a mode
export function getComplianceEnforcementSummary(mode: string): string[] {
    switch (mode?.toUpperCase()) {
        case 'HIPAA':
            return [
                'MFA required for all users',
                'Sessions auto-expire after 15 minutes',
                'Public sharing disabled',
                'All file access events logged',
            ];
        case 'SOX':
        case 'SOC2':
            return [
                'MFA required for all users',
                'File versioning enabled (no overwrites)',
                'Public sharing disabled',
                'All changes to documents and permissions logged',
                'Minimum 365-day retention required',
            ];
        case 'GDPR':
            return [
                'Data deletion requests cannot be blocked',
                'User consent tracking required',
                'All data exports logged',
                'Automatic deletion on retention expiry',
            ];
        default:
            return [];
    }
}
