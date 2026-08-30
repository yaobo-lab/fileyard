import { ArrowLeft, Shield, Database, Eye, Lock, Clock, Users, Globe } from 'lucide-react';
import { Link } from 'react-router-dom';
import { useSettings } from '../context/SettingsContext';
import { useGlobalSettings } from '../context/GlobalSettingsContext';

export default function PrivacyPolicy() {
    const { complianceMode, encryptionStandard, restrictions } = useSettings();
    const { settings } = useGlobalSettings();

    // If custom content is set, render it
    if (settings.privacy_content && settings.privacy_content.trim()) {
        return (
            <div className="max-w-4xl mx-auto">
                <Link 
                    to="/" 
                    className="inline-flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white mb-6"
                >
                    <ArrowLeft className="w-4 h-4" />
                    Back to Dashboard
                </Link>

                <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700">
                    <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                        <div className="flex items-center gap-3">
                            <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-lg">
                                <Shield className="w-6 h-6 text-primary-600 dark:text-primary-400" />
                            </div>
                            <div>
                                <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Privacy Policy</h1>
                            </div>
                        </div>
                    </div>
                    <div 
                        className="p-6 prose dark:prose-invert max-w-none"
                        dangerouslySetInnerHTML={{ __html: settings.privacy_content }}
                    />
                </div>
            </div>
        );
    }

    // Default content
    return (
        <div className="max-w-4xl mx-auto">
            <Link 
                to="/" 
                className="inline-flex items-center gap-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white mb-6"
            >
                <ArrowLeft className="w-4 h-4" />
                Back to Dashboard
            </Link>

            <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700">
                <div className="p-6 border-b border-gray-200 dark:border-gray-700">
                    <div className="flex items-center gap-3">
                        <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-lg">
                            <Shield className="w-6 h-6 text-primary-600 dark:text-primary-400" />
                        </div>
                        <div>
                            <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Privacy Policy</h1>
                            <p className="text-sm text-gray-500 dark:text-gray-400">Last updated: December 2024</p>
                        </div>
                    </div>
                </div>

                <div className="p-6 space-y-8">
                    {/* Introduction */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3">Introduction</h2>
                        <p className="text-gray-600 dark:text-gray-400 leading-relaxed">
                            ClovaLink is an open source document management system. This privacy policy explains how your 
                            self-hosted or managed instance of ClovaLink collects, uses, and protects your data. As an 
                            open source project, you have full visibility into and control over how your data is handled.
                        </p>
                    </section>

                    {/* Data We Collect */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                            <Database className="w-5 h-5 text-gray-400" />
                            Data We Collect
                        </h2>
                        <div className="space-y-4">
                            <div className="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-4">
                                <h3 className="font-medium text-gray-900 dark:text-white mb-2">Account Information</h3>
                                <ul className="text-sm text-gray-600 dark:text-gray-400 space-y-1 list-disc list-inside">
                                    <li>Name and email address</li>
                                    <li>Role and department assignments</li>
                                    <li>Authentication credentials (securely hashed)</li>
                                    <li>MFA configuration (if enabled)</li>
                                </ul>
                            </div>
                            <div className="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-4">
                                <h3 className="font-medium text-gray-900 dark:text-white mb-2">Files and Documents</h3>
                                <ul className="text-sm text-gray-600 dark:text-gray-400 space-y-1 list-disc list-inside">
                                    <li>Uploaded files and their metadata (name, size, type)</li>
                                    <li>File versions and revision history</li>
                                    <li>Folder structure and organization</li>
                                    <li>File sharing and access permissions</li>
                                </ul>
                            </div>
                            <div className="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-4">
                                <h3 className="font-medium text-gray-900 dark:text-white mb-2">Activity Logs</h3>
                                <ul className="text-sm text-gray-600 dark:text-gray-400 space-y-1 list-disc list-inside">
                                    <li>Login and authentication events</li>
                                    <li>File access, upload, download, and modification events</li>
                                    <li>User and permission changes</li>
                                    <li>System and settings modifications</li>
                                </ul>
                            </div>
                        </div>
                    </section>

                    {/* Data Security */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                            <Lock className="w-5 h-5 text-gray-400" />
                            Data Security
                        </h2>
                        <div className="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-4">
                            <p className="text-gray-700 dark:text-gray-300 leading-relaxed mb-3">
                                Your data is protected using industry-standard security measures:
                            </p>
                            <ul className="text-sm text-gray-600 dark:text-gray-400 space-y-2">
                                <li className="flex items-center gap-2">
                                    <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                                    <strong>{encryptionStandard}</strong> encryption for data at rest
                                </li>
                                <li className="flex items-center gap-2">
                                    <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                                    TLS 1.3 encryption for data in transit
                                </li>
                                <li className="flex items-center gap-2">
                                    <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                                    Secure password hashing (Argon2)
                                </li>
                                <li className="flex items-center gap-2">
                                    <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                                    Role-based access control (RBAC)
                                </li>
                                {restrictions?.mfa_required && (
                                    <li className="flex items-center gap-2">
                                        <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                                        Multi-factor authentication enforced
                                    </li>
                                )}
                            </ul>
                        </div>
                    </section>

                    {/* Data Retention */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                            <Clock className="w-5 h-5 text-gray-400" />
                            Data Retention
                        </h2>
                        <p className="text-gray-600 dark:text-gray-400 leading-relaxed">
                            Data retention policies are configured by your organization's administrator. 
                            {restrictions?.min_retention_days && (
                                <span> Your current compliance mode requires a minimum retention period of <strong>{restrictions.min_retention_days} days</strong>.</span>
                            )}
                            {' '}Deleted files are moved to the recycle bin and permanently removed according to your retention settings.
                        </p>
                    </section>

                    {/* Compliance-Specific Section */}
                    {complianceMode && complianceMode !== 'Standard' && complianceMode !== 'None' && (
                        <section>
                            <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                                <Eye className="w-5 h-5 text-gray-400" />
                                {complianceMode} Compliance
                            </h2>
                            <div className={`rounded-lg p-4 border ${
                                complianceMode === 'HIPAA' 
                                    ? 'bg-red-50 dark:bg-red-900/20 border-red-200 dark:border-red-800' 
                                    : complianceMode === 'SOX'
                                    ? 'bg-blue-50 dark:bg-blue-900/20 border-blue-200 dark:border-blue-800'
                                    : 'bg-purple-50 dark:bg-purple-900/20 border-purple-200 dark:border-purple-800'
                            }`}>
                                {complianceMode === 'HIPAA' && (
                                    <div className="text-gray-700 dark:text-gray-300">
                                        <p className="mb-2">This system is configured to comply with the Health Insurance Portability and Accountability Act (HIPAA):</p>
                                        <ul className="text-sm space-y-1 list-disc list-inside">
                                            <li>All Protected Health Information (PHI) access is logged</li>
                                            <li>Automatic session timeout after inactivity</li>
                                            <li>Public file sharing is disabled</li>
                                            <li>Full audit trail of all data access</li>
                                            <li>Business Associate Agreement (BAA) compliance supported</li>
                                        </ul>
                                    </div>
                                )}
                                {complianceMode === 'SOX' && (
                                    <div className="text-gray-700 dark:text-gray-300">
                                        <p className="mb-2">This system is configured to comply with the Sarbanes-Oxley Act (SOX):</p>
                                        <ul className="text-sm space-y-1 list-disc list-inside">
                                            <li>Document versioning prevents unauthorized modifications</li>
                                            <li>Complete audit trail of all document changes</li>
                                            <li>Minimum retention periods enforced</li>
                                            <li>Role separation and access controls</li>
                                            <li>Financial record integrity protection</li>
                                        </ul>
                                    </div>
                                )}
                                {complianceMode === 'GDPR' && (
                                    <div className="text-gray-700 dark:text-gray-300">
                                        <p className="mb-2">This system is configured to comply with the General Data Protection Regulation (GDPR):</p>
                                        <ul className="text-sm space-y-1 list-disc list-inside">
                                            <li>Right to access your personal data</li>
                                            <li>Right to erasure (right to be forgotten)</li>
                                            <li>Data export functionality available</li>
                                            <li>Consent tracking for data processing</li>
                                            <li>Data processing activity logging</li>
                                        </ul>
                                    </div>
                                )}
                            </div>
                        </section>
                    )}

                    {/* Your Rights */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                            <Users className="w-5 h-5 text-gray-400" />
                            Your Rights
                        </h2>
                        <p className="text-gray-600 dark:text-gray-400 leading-relaxed mb-3">
                            Depending on your jurisdiction and applicable regulations, you may have the following rights:
                        </p>
                        <ul className="text-gray-600 dark:text-gray-400 space-y-2 list-disc list-inside">
                            <li>Access your personal data stored in the system</li>
                            <li>Request correction of inaccurate data</li>
                            <li>Request deletion of your data (subject to retention requirements)</li>
                            <li>Export your data in a portable format</li>
                            <li>Object to certain types of data processing</li>
                        </ul>
                        <p className="text-gray-600 dark:text-gray-400 leading-relaxed mt-3">
                            Contact your organization's administrator to exercise these rights.
                        </p>
                    </section>

                    {/* Open Source */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                            <Globe className="w-5 h-5 text-gray-400" />
                            Open Source Transparency
                        </h2>
                        <p className="text-gray-600 dark:text-gray-400 leading-relaxed">
                            ClovaLink is open source software. You can review our source code, security practices, 
                            and data handling procedures at{' '}
                            <a 
                                href="https://github.com/clovalink/clovalink" 
                                target="_blank" 
                                rel="noopener noreferrer"
                                className="text-primary-600 dark:text-primary-400 hover:underline"
                            >
                                github.com/clovalink/clovalink
                            </a>
                            . We believe in transparency and community-driven security.
                        </p>
                    </section>

                    {/* Contact */}
                    <section className="border-t border-gray-200 dark:border-gray-700 pt-6">
                        <p className="text-sm text-gray-500 dark:text-gray-400">
                            For privacy-related inquiries about this instance, contact your organization's administrator. 
                            For questions about the ClovaLink project, visit{' '}
                            <a 
                                href="https://clovalink.org" 
                                target="_blank" 
                                rel="noopener noreferrer"
                                className="text-primary-600 dark:text-primary-400 hover:underline"
                            >
                                clovalink.org
                            </a>.
                        </p>
                    </section>
                </div>
            </div>
        </div>
    );
}
