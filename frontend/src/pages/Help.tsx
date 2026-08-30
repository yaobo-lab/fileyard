import { Shield, Clock, FileText, AlertTriangle, ArrowLeft, HelpCircle } from 'lucide-react';
import { Link } from 'react-router-dom';
import { useGlobalSettings } from '../context/GlobalSettingsContext';

export function Help() {
    const { settings } = useGlobalSettings();

    // If custom content is set, render it
    if (settings.help_content && settings.help_content.trim()) {
        return (
            <div className="max-w-4xl mx-auto space-y-8 p-6">
                <Link 
                    to="/" 
                    className="inline-flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
                >
                    <ArrowLeft className="w-4 h-4" />
                    Back to Dashboard
                </Link>

                <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700">
                    <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                        <div className="flex items-center gap-3">
                            <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-lg">
                                <HelpCircle className="w-6 h-6 text-primary-600 dark:text-primary-400" />
                            </div>
                            <div>
                                <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Help & Documentation</h1>
                            </div>
                        </div>
                    </div>
                    <div 
                        className="p-6 prose dark:prose-invert max-w-none"
                        dangerouslySetInnerHTML={{ __html: settings.help_content }}
                    />
                </div>
            </div>
        );
    }
    
    // Default content
    return (
        <div className="max-w-4xl mx-auto space-y-8 p-6">
            <div>
                <h1 className="text-3xl font-bold text-gray-900 dark:text-white">Help & Documentation</h1>
                <p className="mt-2 text-lg text-gray-600 dark:text-gray-300">
                    Understanding compliance modes and file retention policies.
                </p>
            </div>

            <div className="bg-white dark:bg-gray-800 shadow rounded-lg overflow-hidden">
                <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                    <div className="flex items-center">
                        <Shield className="h-6 w-6 text-primary-600 dark:text-primary-400 mr-3" />
                        <h2 className="text-xl font-semibold text-gray-900 dark:text-white">Compliance Modes</h2>
                    </div>
                    <div className="mt-4 space-y-4">
                        <div className="bg-gray-50 dark:bg-gray-700/50 p-4 rounded-md">
                            <h3 className="font-medium text-gray-900 dark:text-white">HIPAA (Health Insurance Portability and Accountability Act)</h3>
                            <p className="mt-1 text-sm text-gray-600 dark:text-gray-300">
                                Designed for healthcare organizations. Enforces strict access controls, detailed audit logging for PHI access, automatic logout after inactivity, and encryption at rest and in transit.
                            </p>
                        </div>
                        <div className="bg-gray-50 dark:bg-gray-700/50 p-4 rounded-md">
                            <h3 className="font-medium text-gray-900 dark:text-white">SOC2 (Service Organization Control 2)</h3>
                            <p className="mt-1 text-sm text-gray-600 dark:text-gray-300">
                                Focuses on security, availability, processing integrity, confidentiality, and privacy. Requires comprehensive audit trails, change management logging, and security monitoring.
                            </p>
                        </div>
                        <div className="bg-gray-50 dark:bg-gray-700/50 p-4 rounded-md">
                            <h3 className="font-medium text-gray-900 dark:text-white">GDPR (General Data Protection Regulation)</h3>
                            <p className="mt-1 text-sm text-gray-600 dark:text-gray-300">
                                For organizations handling EU citizen data. Emphasizes data privacy, consent management, and the "right to be forgotten" (permanent deletion capabilities).
                            </p>
                        </div>
                    </div>
                </div>

                <div className="p-6">
                    <div className="flex items-center">
                        <Clock className="h-6 w-6 text-primary-600 dark:text-primary-400 mr-3" />
                        <h2 className="text-xl font-semibold text-gray-900 dark:text-white">File Retention Policy</h2>
                    </div>
                    <p className="mt-2 text-gray-600 dark:text-gray-300">
                        Your organization's retention policy determines how long deleted files are kept in the Recycle Bin before being permanently removed from our servers.
                    </p>

                    <div className="mt-6 grid gap-6 md:grid-cols-2">
                        <div className="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                            <h3 className="font-medium text-gray-900 dark:text-white flex items-center">
                                <FileText className="h-4 w-4 mr-2 text-gray-500" />
                                Soft Deletion
                            </h3>
                            <p className="mt-2 text-sm text-gray-600 dark:text-gray-300">
                                When you delete a file, it is moved to the Recycle Bin. It remains recoverable until the retention period expires.
                            </p>
                        </div>
                        <div className="border border-gray-200 dark:border-gray-700 rounded-lg p-4">
                            <h3 className="font-medium text-gray-900 dark:text-white flex items-center">
                                <AlertTriangle className="h-4 w-4 mr-2 text-red-500" />
                                Permanent Deletion
                            </h3>
                            <p className="mt-2 text-sm text-gray-600 dark:text-gray-300">
                                Once the retention period (30, 60, 90, 120, or 365 days) passes, files are automatically and permanently deleted. This action cannot be undone.
                            </p>
                        </div>
                    </div>

                    <div className="mt-6 bg-blue-50 dark:bg-blue-900/20 p-4 rounded-md border border-blue-100 dark:border-blue-800">
                        <h4 className="text-sm font-medium text-blue-800 dark:text-blue-300">Configuring Retention</h4>
                        <p className="mt-1 text-sm text-blue-700 dark:text-blue-400">
                            Administrators can configure the retention period in <strong>Settings &gt; Compliance</strong>. The default retention period is 30 days.
                        </p>
                    </div>
                </div>
            </div>
        </div>
    );
}
