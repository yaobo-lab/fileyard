import { useState, useEffect, useMemo } from 'react';
import { X, Eye, EyeOff, Code, RotateCcw, Save, AlertTriangle, Check, Variable } from 'lucide-react';
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
}

interface EmailTemplateEditorProps {
    template: EmailTemplate | null;
    onSave: (data: { subject: string; body_html: string; body_text?: string }) => Promise<void>;
    onReset?: () => Promise<void>;
    onClose: () => void;
    isLoading?: boolean;
    canReset?: boolean;
}

export function EmailTemplateEditor({
    template,
    onSave,
    onReset,
    onClose,
    isLoading = false,
    canReset = false,
}: EmailTemplateEditorProps) {
    const [subject, setSubject] = useState('');
    const [bodyHtml, setBodyHtml] = useState('');
    const [bodyText, setBodyText] = useState('');
    const [showPreview, setShowPreview] = useState(false);
    const [showHtml, setShowHtml] = useState(true);
    const [isSaving, setIsSaving] = useState(false);
    const [isResetting, setIsResetting] = useState(false);
    const [hasChanges, setHasChanges] = useState(false);

    useEffect(() => {
        if (template) {
            setSubject(template.subject);
            setBodyHtml(template.body_html);
            setBodyText(template.body_text || '');
            setHasChanges(false);
        }
    }, [template]);

    // Sample data for preview
    const sampleData: Record<string, string> = useMemo(() => ({
        user_name: 'John Doe',
        company_name: 'Acme Corp',
        file_name: 'quarterly-report.pdf',
        request_name: 'Q4 Financial Reports',
        uploader_name: 'Jane Smith',
        sharer_name: 'Bob Johnson',
        new_user_name: 'New Employee',
        new_user_email: 'newuser@company.com',
        new_user_role: 'Employee',
        old_role: 'Employee',
        new_role: 'Manager',
        role: 'Employee',
        days_until_expiry: '3',
        percentage_used: '85',
        alert_type: 'Retention Policy Violation',
        message: 'Files older than 30 days found that should have been archived.',
        reset_link: 'https://app.example.com/reset-password?token=xxx',
        user_email: 'user@company.com',
        temp_password: 'TempPass123!',
        app_url: 'https://app.example.com',
    }), []);

    const renderPreview = (text: string) => {
        let result = text;
        Object.entries(sampleData).forEach(([key, value]) => {
            result = result.replace(new RegExp(`\\{\\{${key}\\}\\}`, 'g'), value);
        });
        return result;
    };

    const handleSubjectChange = (value: string) => {
        setSubject(value);
        setHasChanges(true);
    };

    const handleBodyHtmlChange = (value: string) => {
        setBodyHtml(value);
        setHasChanges(true);
    };

    const insertVariable = (variable: string) => {
        const placeholder = `{{${variable}}}`;
        // Insert at cursor position in the HTML body (simple append for now)
        setBodyHtml(prev => prev + placeholder);
        setHasChanges(true);
    };

    const handleSave = async () => {
        setIsSaving(true);
        try {
            await onSave({ subject, body_html: bodyHtml, body_text: bodyText || undefined });
            setHasChanges(false);
        } finally {
            setIsSaving(false);
        }
    };

    const handleReset = async () => {
        if (!onReset) return;
        setIsResetting(true);
        try {
            await onReset();
            // Reset to global values if available
            if (template?.global_subject && template?.global_body_html) {
                setSubject(template.global_subject);
                setBodyHtml(template.global_body_html);
            }
            setHasChanges(false);
        } finally {
            setIsResetting(false);
        }
    };

    if (!template) return null;

    return (
        <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-2">
            <div className="bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-7xl h-[95vh] flex flex-col">
                {/* Header */}
                <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                    <div>
                        <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
                            Edit: {template.name}
                        </h2>
                        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                            Template Key: <code className="bg-gray-100 dark:bg-gray-700 px-1.5 py-0.5 rounded text-xs">{template.template_key}</code>
                            {template.is_customized && (
                                <span className="ml-2 inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300">
                                    Customized
                                </span>
                            )}
                        </p>
                    </div>
                    <button
                        onClick={onClose}
                        className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700"
                    >
                        <X className="w-5 h-5" />
                    </button>
                </div>

                {/* Content */}
                <div className="flex-1 overflow-hidden flex">
                    {/* Editor Panel */}
                    <div className="flex-1 flex flex-col overflow-hidden border-r border-gray-200 dark:border-gray-700">
                        {/* Subject */}
                        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                                Email Subject
                            </label>
                            <input
                                type="text"
                                value={subject}
                                onChange={(e) => handleSubjectChange(e.target.value)}
                                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent"
                                placeholder="Email subject line..."
                            />
                        </div>

                        {/* Variables Toolbar */}
                        <div className="px-6 py-3 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50">
                            <div className="flex items-center gap-2 flex-wrap">
                                <span className="text-xs font-medium text-gray-500 dark:text-gray-400 flex items-center gap-1">
                                    <Variable className="w-3 h-3" />
                                    Variables:
                                </span>
                                {template.variables.map((variable) => (
                                    <button
                                        key={variable}
                                        onClick={() => insertVariable(variable)}
                                        className="px-2 py-1 text-xs bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300 rounded hover:bg-primary-200 dark:hover:bg-primary-900/50 transition-colors font-mono"
                                    >
                                        {`{{${variable}}}`}
                                    </button>
                                ))}
                            </div>
                        </div>

                        {/* Body Editor */}
                        <div className="flex-1 overflow-hidden flex flex-col min-h-0">
                            <div className="px-6 py-2 flex items-center gap-2 border-b border-gray-200 dark:border-gray-700 flex-shrink-0">
                                <button
                                    onClick={() => setShowHtml(true)}
                                    className={clsx(
                                        "px-3 py-1.5 text-sm font-medium rounded-md flex items-center gap-1.5",
                                        showHtml
                                            ? "bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300"
                                            : "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
                                    )}
                                >
                                    <Code className="w-4 h-4" />
                                    HTML
                                </button>
                                <button
                                    onClick={() => setShowHtml(false)}
                                    className={clsx(
                                        "px-3 py-1.5 text-sm font-medium rounded-md",
                                        !showHtml
                                            ? "bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300"
                                            : "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
                                    )}
                                >
                                    Plain Text (Fallback)
                                </button>
                            </div>
                            <div className="flex-1 overflow-auto p-4 min-h-0">
                                <textarea
                                    value={showHtml ? bodyHtml : bodyText}
                                    onChange={(e) => showHtml ? handleBodyHtmlChange(e.target.value) : setBodyText(e.target.value)}
                                    className="w-full h-full min-h-[400px] px-4 py-3 font-mono text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-900 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-transparent resize-y"
                                    placeholder={showHtml ? "Enter HTML email body..." : "Enter plain text fallback..."}
                                    style={{ minHeight: '400px' }}
                                />
                            </div>
                        </div>
                    </div>

                    {/* Preview Panel */}
                    <div className={clsx(
                        "w-[45%] flex flex-col overflow-hidden transition-all",
                        !showPreview && "hidden"
                    )}>
                        <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50">
                            <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300">
                                Preview (with sample data)
                            </h3>
                        </div>
                        <div className="flex-1 overflow-auto bg-white">
                            <div className="p-4 text-sm text-gray-600 border-b">
                                <strong>Subject:</strong> {renderPreview(subject)}
                            </div>
                            <iframe
                                srcDoc={renderPreview(bodyHtml)}
                                className="w-full h-full border-0"
                                title="Email Preview"
                                sandbox="allow-same-origin"
                            />
                        </div>
                    </div>
                </div>

                {/* Footer */}
                <div className="flex items-center justify-between px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50">
                    <div className="flex items-center gap-3">
                        <button
                            onClick={() => setShowPreview(!showPreview)}
                            className={clsx(
                                "flex items-center gap-2 px-3 py-2 text-sm font-medium rounded-lg transition-colors",
                                showPreview
                                    ? "bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300"
                                    : "text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700"
                            )}
                        >
                            {showPreview ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                            {showPreview ? 'Hide Preview' : 'Show Preview'}
                        </button>
                        
                        {canReset && template.is_customized && (
                            <button
                                onClick={handleReset}
                                disabled={isResetting}
                                className="flex items-center gap-2 px-3 py-2 text-sm font-medium text-amber-600 dark:text-amber-400 hover:bg-amber-50 dark:hover:bg-amber-900/20 rounded-lg transition-colors disabled:opacity-50"
                            >
                                <RotateCcw className={clsx("w-4 h-4", isResetting && "animate-spin")} />
                                Reset to Default
                            </button>
                        )}
                    </div>

                    <div className="flex items-center gap-3">
                        {hasChanges && (
                            <span className="flex items-center gap-1 text-sm text-amber-600 dark:text-amber-400">
                                <AlertTriangle className="w-4 h-4" />
                                Unsaved changes
                            </span>
                        )}
                        <button
                            onClick={onClose}
                            className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                        >
                            Cancel
                        </button>
                        <button
                            onClick={handleSave}
                            disabled={isSaving || isLoading || !hasChanges}
                            className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            {isSaving ? (
                                <>
                                    <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                                    Saving...
                                </>
                            ) : (
                                <>
                                    <Save className="w-4 h-4" />
                                    Save Changes
                                </>
                            )}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}

