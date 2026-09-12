import { useState, useEffect, useCallback, ReactNode } from 'react';
import { 
    Mail, Edit2, Check, AlertTriangle, Upload, Clock, UserPlus, RefreshCw, 
    Share2, ShieldAlert, HardDrive, KeyRound, UserCheck, LucideIcon 
} from 'lucide-react';
import { useAuthFetch } from '../../context/AuthContext';
import { EmailTemplateEditor } from '../../components/EmailTemplateEditor';
import { useTranslations } from '../../context/I18nContext';
import clsx from 'clsx';

interface EmailTemplate {
    template_key: string;
    name: string;
    subject: string;
    body_html: string;
    body_text?: string | null;
    variables: string[];
    is_customized?: boolean;
    global_subject?: string;
    global_body_html?: string;
    updated_at?: string;
}

// Template category icons and colors
const TEMPLATE_CATEGORIES: Record<string, { Icon: LucideIcon; color: string }> = {
    file_upload: { Icon: Upload, color: 'blue' },
    request_expiring: { Icon: Clock, color: 'amber' },
    user_created: { Icon: UserPlus, color: 'green' },
    role_changed: { Icon: RefreshCw, color: 'purple' },
    file_shared: { Icon: Share2, color: 'blue' },
    compliance_alert: { Icon: ShieldAlert, color: 'red' },
    storage_warning: { Icon: HardDrive, color: 'orange' },
    password_reset: { Icon: KeyRound, color: 'indigo' },
    welcome: { Icon: UserCheck, color: 'teal' },
};

export function EmailTemplatesSettings() {
    const authFetch = useAuthFetch();
    const t = useTranslations('SettingsEmail');
    const tCommon = useTranslations('Common');
    const [templates, setTemplates] = useState<EmailTemplate[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [editingTemplate, setEditingTemplate] = useState<EmailTemplate | null>(null);
    const [successMessage, setSuccessMessage] = useState<string | null>(null);

    const getTemplateDesc = (key: string) => {
        switch (key) {
            case 'file_upload': return t('descFileUpload');
            case 'request_expiring': return t('descRequestExpiring');
            case 'user_created': return t('descUserCreated');
            case 'role_changed': return t('descRoleChanged');
            case 'file_shared': return t('descFileShared');
            case 'compliance_alert': return t('descComplianceAlert');
            case 'storage_warning': return t('descStorageWarning');
            case 'password_reset': return t('descPasswordReset');
            case 'welcome': return t('descWelcome');
            default: return t('descDefault');
        }
    };

    const fetchTemplates = useCallback(async () => {
        setIsLoading(true);
        setError(null);
        try {
            const response = await authFetch('/api/email-templates');
            if (!response.ok) throw new Error('Failed to fetch templates');
            const data = await response.json();
            // Map the response to include variables as array
            const mapped = data.map((t: any) => ({
                ...t,
                variables: Array.isArray(t.variables) ? t.variables : [],
            }));
            setTemplates(mapped);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to load templates');
        } finally {
            setIsLoading(false);
        }
    }, [authFetch]);

    useEffect(() => {
        fetchTemplates();
    }, [fetchTemplates]);

    const handleSave = async (data: { subject: string; body_html: string; body_text?: string }) => {
        if (!editingTemplate) return;
        
        try {
            const response = await authFetch(`/api/email-templates/${editingTemplate.template_key}`, {
                method: 'PUT',
                body: JSON.stringify(data),
            });
            
            if (!response.ok) throw new Error('Failed to save template');
            
            setSuccessMessage(t('saveSuccess', { name: editingTemplate.name }));
            setTimeout(() => setSuccessMessage(null), 3000);
            setEditingTemplate(null);
            fetchTemplates();
        } catch (err) {
            throw err;
        }
    };

    const getTemplateInfo = (key: string) => {
        return TEMPLATE_CATEGORIES[key] || { Icon: Mail, color: 'gray' };
    };

    if (isLoading) {
        return (
            <div className="flex items-center justify-center py-12">
                <div className="w-8 h-8 border-4 border-primary-200 border-t-primary-600 rounded-full animate-spin" />
            </div>
        );
    }

    return (
        <div className="space-y-6">
            {/* Header */}
            <div>
                <h2 className="text-lg font-semibold text-gray-900 dark:text-white flex items-center gap-2">
                    <Mail className="w-5 h-5 text-primary-500" />
                    {t('title')}
                </h2>
                <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
                    {t('description')}
                </p>
            </div>

            {/* Success Message */}
            {successMessage && (
                <div className="flex items-center gap-2 p-3 bg-green-50 dark:bg-green-900/20 text-green-700 dark:text-green-300 rounded-lg">
                    <Check className="w-5 h-5" />
                    {successMessage}
                </div>
            )}

            {/* Error Message */}
            {error && (
                <div className="flex items-center gap-2 p-3 bg-red-50 dark:bg-red-900/20 text-red-700 dark:text-red-300 rounded-lg">
                    <AlertTriangle className="w-5 h-5" />
                    {error}
                </div>
            )}

            {/* Templates List */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden">
                {templates.length === 0 ? (
                    <div className="p-8 text-center text-gray-500 dark:text-gray-400 text-sm">
                        {t('noTemplates')}
                    </div>
                ) : (
                    <div className="divide-y divide-gray-200 dark:divide-gray-700">
                        {templates.map((template) => {
                            const info = getTemplateInfo(template.template_key);
                            const IconComponent = info.Icon;
                            return (
                                <div
                                    key={template.template_key}
                                    className="p-4 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
                                >
                                    <div className="flex items-start justify-between">
                                        <div className="flex items-start gap-3">
                                            <div className={clsx(
                                                "w-10 h-10 rounded-lg flex items-center justify-center",
                                                info.color === 'blue' && "bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400",
                                                info.color === 'amber' && "bg-amber-100 dark:bg-amber-900/30 text-amber-600 dark:text-amber-400",
                                                info.color === 'green' && "bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400",
                                                info.color === 'purple' && "bg-purple-100 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400",
                                                info.color === 'red' && "bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400",
                                                info.color === 'orange' && "bg-orange-100 dark:bg-orange-900/30 text-orange-600 dark:text-orange-400",
                                                info.color === 'indigo' && "bg-indigo-100 dark:bg-indigo-900/30 text-indigo-600 dark:text-indigo-400",
                                                info.color === 'teal' && "bg-teal-100 dark:bg-teal-900/30 text-teal-600 dark:text-teal-400",
                                                info.color === 'gray' && "bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400"
                                            )}>
                                                <IconComponent className="w-5 h-5" />
                                            </div>
                                            <div>
                                                <h3 className="font-medium text-gray-900 dark:text-white">
                                                    {template.name}
                                                </h3>
                                                <p className="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
                                                    {getTemplateDesc(template.template_key)}
                                                </p>
                                                <p className="text-xs text-gray-400 dark:text-gray-500 mt-1 font-mono">
                                                    {t('key')}: {template.template_key}
                                                </p>
                                            </div>
                                        </div>
                                        <button
                                            onClick={() => setEditingTemplate(template)}
                                            className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-primary-600 dark:text-primary-400 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-lg transition-colors"
                                        >
                                            <Edit2 className="w-4 h-4" />
                                            {tCommon('edit')}
                                        </button>
                                    </div>
                                    
                                    {/* Subject Preview */}
                                    <div className="mt-3 ml-11">
                                        <div className="text-xs font-medium text-gray-500 dark:text-gray-400 mb-1">
                                            {t('subject')}:
                                        </div>
                                        <div className="text-sm text-gray-700 dark:text-gray-300 bg-gray-50 dark:bg-gray-900/50 px-3 py-2 rounded-md font-mono">
                                            {template.subject}
                                        </div>
                                    </div>

                                {/* Variables */}
                                {template.variables && template.variables.length > 0 && (
                                    <div className="mt-2 ml-11 flex flex-wrap gap-1">
                                        {template.variables.map((v: string) => (
                                            <span
                                                key={v}
                                                className="px-1.5 py-0.5 text-xs bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded font-mono"
                                            >
                                                {`{{${v}}}`}
                                            </span>
                                        ))}
                                    </div>
                                )}
                            </div>
                        );
                    })}
                </div>
            )}
            </div>

            {/* Editor Modal */}
            {editingTemplate && (
                <EmailTemplateEditor
                    template={editingTemplate}
                    onSave={handleSave}
                    onClose={() => setEditingTemplate(null)}
                    canReset={false}
                />
            )}
        </div>
    );
}

