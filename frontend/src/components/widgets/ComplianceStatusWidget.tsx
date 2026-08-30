import { Shield, ShieldCheck, ShieldAlert, Lock, Eye, Clock, FileText } from 'lucide-react';
import { useSettings } from '../../context/SettingsContext';

export function ComplianceStatusWidget() {
    const { complianceMode, restrictions, isComplianceActive } = useSettings();

    const getComplianceColor = () => {
        switch (complianceMode) {
            case 'HIPAA':
                return 'text-purple-600 dark:text-purple-400 bg-purple-100 dark:bg-purple-900/30';
            case 'SOX':
                return 'text-blue-600 dark:text-blue-400 bg-blue-100 dark:bg-blue-900/30';
            case 'GDPR':
                return 'text-green-600 dark:text-green-400 bg-green-100 dark:bg-green-900/30';
            default:
                return 'text-gray-600 dark:text-gray-400 bg-gray-100 dark:bg-gray-700';
        }
    };

    const getEnforcedSettings = () => {
        if (!restrictions) return [];
        
        const settings = [];
        if (restrictions.mfa_required) settings.push({ icon: Lock, label: 'MFA Required', active: true });
        if (restrictions.audit_logging_mandatory) settings.push({ icon: Eye, label: 'Audit Logging', active: true });
        if (restrictions.public_sharing_blocked) settings.push({ icon: ShieldAlert, label: 'Public Sharing Blocked', active: true });
        if (restrictions.session_timeout_minutes) settings.push({ icon: Clock, label: `${restrictions.session_timeout_minutes}min Session Timeout`, active: true });
        if (restrictions.file_versioning_required) settings.push({ icon: FileText, label: 'File Versioning', active: true });
        
        return settings;
    };

    const enforcedSettings = getEnforcedSettings();

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-6 h-full">
            <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-medium text-gray-900 dark:text-white">Compliance Status</h3>
                <Shield className="w-5 h-5 text-gray-400" />
            </div>
            
            <div className="space-y-4">
                {/* Current Mode */}
                <div className={`flex items-center p-4 rounded-lg ${getComplianceColor()}`}>
                    <ShieldCheck className="w-8 h-8 mr-3" />
                    <div>
                        <p className="text-lg font-semibold">
                            {complianceMode === 'Standard' || complianceMode === 'None' ? 'Standard Mode' : `${complianceMode} Compliant`}
                        </p>
                        <p className="text-sm opacity-75">
                            {isComplianceActive ? 'Active enforcement' : 'No restrictions'}
                        </p>
                    </div>
                </div>

                {/* Enforced Settings */}
                {enforcedSettings.length > 0 && (
                    <div>
                        <p className="text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">
                            Enforced Settings
                        </p>
                        <div className="space-y-2">
                            {enforcedSettings.map((setting, index) => {
                                const Icon = setting.icon;
                                return (
                                    <div key={index} className="flex items-center text-sm text-gray-700 dark:text-gray-300">
                                        <Icon className="w-4 h-4 mr-2 text-green-500" />
                                        {setting.label}
                                    </div>
                                );
                            })}
                        </div>
                    </div>
                )}

                {enforcedSettings.length === 0 && isComplianceActive && (
                    <p className="text-sm text-gray-500 dark:text-gray-400 text-center py-2">
                        No special restrictions enforced
                    </p>
                )}
            </div>
        </div>
    );
}
