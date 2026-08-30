import { useState, useEffect } from 'react';
import { Save, Check, Loader2, FileText, Shield, HelpCircle, Eye, RotateCcw } from 'lucide-react';
import { useGlobalSettings } from '../../context/GlobalSettingsContext';
import clsx from 'clsx';

type PageType = 'tos' | 'privacy' | 'help';

// Default content that matches what's displayed on the actual pages
const DEFAULT_TOS_CONTENT = `<h2>Agreement to Terms</h2>
<p>By accessing or using ClovaLink, you agree to be bound by these Terms of Service. ClovaLink is open source software provided under the terms of its license agreement. Your use of this software is also subject to any additional terms set by your organization's administrator.</p>

<h2>Open Source License</h2>
<p>ClovaLink is released under an open source license. You are free to:</p>
<ul>
<li>Use the software for any purpose, including commercial use</li>
<li>Modify the source code to suit your needs</li>
<li>Distribute copies of the original or modified software</li>
<li>Self-host on your own infrastructure</li>
</ul>
<p>Full license details are available in the project repository at <a href="https://github.com/clovalink/clovalink">github.com/clovalink/clovalink</a></p>

<h2>Disclaimer of Warranties</h2>
<p><strong>THIS SOFTWARE IS PROVIDED "AS IS"</strong></p>
<p>THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS, COPYRIGHT HOLDERS, OR CONTRIBUTORS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.</p>

<h2>Limitation of Liability</h2>
<p>To the maximum extent permitted by applicable law, in no event shall ClovaLink, its authors, contributors, or affiliated organizations be liable for:</p>
<ul>
<li>Any indirect, incidental, special, consequential, or punitive damages</li>
<li>Loss of profits, data, use, goodwill, or other intangible losses</li>
<li>Any damages resulting from unauthorized access to or alteration of your data</li>
<li>Any interruption or cessation of functionality</li>
<li>Any bugs, viruses, or other harmful code transmitted through the software</li>
<li>Any errors or omissions in any content</li>
</ul>
<p>This limitation applies regardless of the legal theory under which such damages are sought.</p>

<h2>User Responsibilities</h2>
<p>As a user of ClovaLink, you agree to:</p>
<ul>
<li>Maintain the confidentiality of your account credentials</li>
<li>Use the software in compliance with applicable laws and regulations</li>
<li>Not attempt to circumvent security measures or access controls</li>
<li>Not upload malicious content or engage in harmful activities</li>
<li>Respect the intellectual property rights of others</li>
<li>Comply with your organization's acceptable use policies</li>
</ul>

<h2>Compliance Features Disclaimer</h2>
<p>ClovaLink provides features designed to assist with regulatory compliance (HIPAA, SOX, GDPR, etc.). However, <strong>use of these features does not guarantee compliance</strong> with any specific regulation. Compliance depends on proper configuration, organizational policies, and adherence to applicable requirements. Consult with qualified legal and compliance professionals to ensure your use meets regulatory requirements.</p>

<h2>Data Responsibility</h2>
<p>For self-hosted installations, the organization hosting the software is solely responsible for:</p>
<ul>
<li>Data backup and recovery procedures</li>
<li>Security configuration and maintenance</li>
<li>Compliance with data protection regulations</li>
<li>Access control and user management</li>
<li>Infrastructure security and updates</li>
</ul>

<h2>Modifications to Terms</h2>
<p>These terms may be updated periodically. Continued use of ClovaLink after changes constitutes acceptance of the modified terms. Check this page regularly for updates.</p>

<h2>Governing Law</h2>
<p>These terms shall be governed by and construed in accordance with the laws of the jurisdiction in which you operate, without regard to its conflict of law provisions.</p>

<p><em>For questions about these terms regarding this instance, contact your organization's administrator. For questions about the ClovaLink project, visit <a href="https://clovalink.org">clovalink.org</a>.</em></p>`;

const DEFAULT_PRIVACY_CONTENT = `<h2>Introduction</h2>
<p>ClovaLink is an open source document management system. This privacy policy explains how your self-hosted or managed instance of ClovaLink collects, uses, and protects your data. As an open source project, you have full visibility into and control over how your data is handled.</p>

<h2>Data We Collect</h2>
<h3>Account Information</h3>
<ul>
<li>Name and email address</li>
<li>Role and department assignments</li>
<li>Authentication credentials (securely hashed)</li>
<li>MFA configuration (if enabled)</li>
</ul>

<h3>Files and Documents</h3>
<ul>
<li>Uploaded files and their metadata (name, size, type)</li>
<li>File versions and revision history</li>
<li>Folder structure and organization</li>
<li>File sharing and access permissions</li>
</ul>

<h3>Activity Logs</h3>
<ul>
<li>Login and authentication events</li>
<li>File access, upload, download, and modification events</li>
<li>User and permission changes</li>
<li>System and settings modifications</li>
</ul>

<h2>Data Security</h2>
<p>Your data is protected using industry-standard security measures:</p>
<ul>
<li>AES-256 encryption for data at rest</li>
<li>TLS 1.3 encryption for data in transit</li>
<li>Secure password hashing (Argon2)</li>
<li>Role-based access control (RBAC)</li>
</ul>

<h2>Data Retention</h2>
<p>Data retention policies are configured by your organization's administrator. Deleted files are moved to the recycle bin and permanently removed according to your retention settings.</p>

<h2>Your Rights</h2>
<p>Depending on your jurisdiction and applicable regulations, you may have the following rights:</p>
<ul>
<li>Access your personal data stored in the system</li>
<li>Request correction of inaccurate data</li>
<li>Request deletion of your data (subject to retention requirements)</li>
<li>Export your data in a portable format</li>
<li>Object to certain types of data processing</li>
</ul>
<p>Contact your organization's administrator to exercise these rights.</p>

<h2>Open Source Transparency</h2>
<p>ClovaLink is open source software. You can review our source code, security practices, and data handling procedures at <a href="https://github.com/clovalink/clovalink">github.com/clovalink/clovalink</a>. We believe in transparency and community-driven security.</p>

<p><em>For privacy-related inquiries about this instance, contact your organization's administrator. For questions about the ClovaLink project, visit <a href="https://clovalink.org">clovalink.org</a>.</em></p>`;

const DEFAULT_HELP_CONTENT = `<h2>Compliance Modes</h2>

<h3>HIPAA (Health Insurance Portability and Accountability Act)</h3>
<p>Designed for healthcare organizations. Enforces strict access controls, detailed audit logging for PHI access, automatic logout after inactivity, and encryption at rest and in transit.</p>

<h3>SOC2 (Service Organization Control 2)</h3>
<p>Focuses on security, availability, processing integrity, confidentiality, and privacy. Requires comprehensive audit trails, change management logging, and security monitoring.</p>

<h3>GDPR (General Data Protection Regulation)</h3>
<p>For organizations handling EU citizen data. Emphasizes data privacy, consent management, and the "right to be forgotten" (permanent deletion capabilities).</p>

<h2>File Retention Policy</h2>
<p>Your organization's retention policy determines how long deleted files are kept in the Recycle Bin before being permanently removed from our servers.</p>

<h3>Soft Deletion</h3>
<p>When you delete a file, it is moved to the Recycle Bin. It remains recoverable until the retention period expires.</p>

<h3>Permanent Deletion</h3>
<p>Once the retention period (30, 60, 90, 120, or 365 days) passes, files are automatically and permanently deleted. This action cannot be undone.</p>

<h3>Configuring Retention</h3>
<p>Administrators can configure the retention period in <strong>Settings &gt; Compliance</strong>. The default retention period is 30 days.</p>

<h2>Getting Started</h2>
<h3>Uploading Files</h3>
<p>Navigate to Files, then drag and drop files or click "Upload" to select files from your computer.</p>

<h3>Creating Folders</h3>
<p>Click "New Folder" to organize your files into folders.</p>

<h3>Sharing Files</h3>
<p>Right-click a file and select "Share" to share with other users or create a public link.</p>

<h3>File Requests</h3>
<p>Use File Requests to receive files from external users without giving them full account access.</p>`;

const DEFAULT_CONTENT: Record<PageType, string> = {
    tos: DEFAULT_TOS_CONTENT,
    privacy: DEFAULT_PRIVACY_CONTENT,
    help: DEFAULT_HELP_CONTENT,
};

const PAGES = [
    { 
        id: 'tos' as PageType, 
        label: 'Terms of Service', 
        icon: FileText,
        description: 'Legal terms and conditions for using the platform',
        settingKey: 'tos_content' as const,
    },
    { 
        id: 'privacy' as PageType, 
        label: 'Privacy Policy', 
        icon: Shield,
        description: 'How user data is collected, used, and protected',
        settingKey: 'privacy_content' as const,
    },
    { 
        id: 'help' as PageType, 
        label: 'Quickstart / Help', 
        icon: HelpCircle,
        description: 'Getting started guide and help documentation',
        settingKey: 'help_content' as const,
    },
];

export function PagesSettings() {
    const { settings, updateSettings } = useGlobalSettings();
    
    const [activePage, setActivePage] = useState<PageType>('tos');
    const [tosContent, setTosContent] = useState('');
    const [privacyContent, setPrivacyContent] = useState('');
    const [helpContent, setHelpContent] = useState('');
    const [isSaving, setIsSaving] = useState(false);
    const [saveSuccess, setSaveSuccess] = useState(false);
    const [showPreview, setShowPreview] = useState(false);

    // Initialize with default content if database is empty
    useEffect(() => {
        setTosContent(settings.tos_content?.trim() || DEFAULT_CONTENT.tos);
        setPrivacyContent(settings.privacy_content?.trim() || DEFAULT_CONTENT.privacy);
        setHelpContent(settings.help_content?.trim() || DEFAULT_CONTENT.help);
    }, [settings]);

    const getContent = (page: PageType) => {
        switch (page) {
            case 'tos': return tosContent;
            case 'privacy': return privacyContent;
            case 'help': return helpContent;
        }
    };

    const getOriginalContent = (page: PageType) => {
        switch (page) {
            case 'tos': return settings.tos_content?.trim() || DEFAULT_CONTENT.tos;
            case 'privacy': return settings.privacy_content?.trim() || DEFAULT_CONTENT.privacy;
            case 'help': return settings.help_content?.trim() || DEFAULT_CONTENT.help;
        }
    };

    const setContent = (page: PageType, content: string) => {
        switch (page) {
            case 'tos': setTosContent(content); break;
            case 'privacy': setPrivacyContent(content); break;
            case 'help': setHelpContent(content); break;
        }
    };

    const resetToDefault = (page: PageType) => {
        setContent(page, DEFAULT_CONTENT[page]);
    };

    const hasChanges = 
        tosContent !== getOriginalContent('tos') ||
        privacyContent !== getOriginalContent('privacy') ||
        helpContent !== getOriginalContent('help');

    const isUsingDefault = (page: PageType) => {
        return getContent(page) === DEFAULT_CONTENT[page];
    };

    const handleSave = async () => {
        setIsSaving(true);
        setSaveSuccess(false);
        
        const success = await updateSettings({
            tos_content: tosContent,
            privacy_content: privacyContent,
            help_content: helpContent,
        });
        
        setIsSaving(false);
        if (success) {
            setSaveSuccess(true);
            setTimeout(() => setSaveSuccess(false), 3000);
        }
    };

    const activePageInfo = PAGES.find(p => p.id === activePage)!;

    return (
        <div className="space-y-6">
            <div className="flex items-center justify-between">
                <div>
                    <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Page Content</h2>
                    <p className="text-sm text-gray-500 dark:text-gray-400">Edit the content of legal and help pages</p>
                </div>
                <button
                    onClick={handleSave}
                    disabled={!hasChanges || isSaving}
                    className={clsx(
                        "flex items-center px-4 py-2 rounded-lg text-sm font-medium transition-all",
                        hasChanges && !isSaving
                            ? "bg-primary-600 text-white hover:bg-primary-700"
                            : "bg-gray-100 dark:bg-gray-700 text-gray-400 cursor-not-allowed"
                    )}
                >
                    {isSaving ? (
                        <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                    ) : saveSuccess ? (
                        <Check className="w-4 h-4 mr-2" />
                    ) : (
                        <Save className="w-4 h-4 mr-2" />
                    )}
                    {isSaving ? 'Saving...' : saveSuccess ? 'Saved!' : 'Save All'}
                </button>
            </div>

            {/* Page Selector */}
            <div className="flex gap-2">
                {PAGES.map((page) => {
                    const Icon = page.icon;
                    return (
                        <button
                            key={page.id}
                            onClick={() => setActivePage(page.id)}
                            className={clsx(
                                "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all",
                                activePage === page.id
                                    ? "bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-400"
                                    : "bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-600"
                            )}
                        >
                            <Icon className="w-4 h-4" />
                            {page.label}
                        </button>
                    );
                })}
            </div>

            {/* Editor */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
                    <div className="flex items-center gap-3">
                        <activePageInfo.icon className="w-5 h-5 text-gray-500 dark:text-gray-400" />
                        <div>
                            <h3 className="font-medium text-gray-900 dark:text-white">{activePageInfo.label}</h3>
                            <p className="text-xs text-gray-500 dark:text-gray-400">{activePageInfo.description}</p>
                        </div>
                    </div>
                    <div className="flex items-center gap-2">
                        {!isUsingDefault(activePage) && (
                            <button
                                onClick={() => resetToDefault(activePage)}
                                className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-600 transition-colors"
                                title="Reset to default content"
                            >
                                <RotateCcw className="w-4 h-4" />
                                Reset
                            </button>
                        )}
                        <button
                            onClick={() => setShowPreview(!showPreview)}
                            className={clsx(
                                "flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm transition-colors",
                                showPreview
                                    ? "bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-400"
                                    : "bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 hover:bg-gray-200 dark:hover:bg-gray-600"
                            )}
                        >
                            <Eye className="w-4 h-4" />
                            {showPreview ? 'Edit' : 'Preview'}
                        </button>
                    </div>
                </div>
                <div className="p-6">
                    {showPreview ? (
                        <div 
                            className="prose dark:prose-invert max-w-none min-h-[400px] p-4 bg-gray-50 dark:bg-gray-900 rounded-lg"
                            dangerouslySetInnerHTML={{ 
                                __html: getContent(activePage) || '<p class="text-gray-400">No content yet. Switch to edit mode to add content.</p>' 
                            }}
                        />
                    ) : (
                        <div className="space-y-4">
                            <textarea
                                value={getContent(activePage)}
                                onChange={(e) => setContent(activePage, e.target.value)}
                                rows={20}
                                placeholder={`Enter ${activePageInfo.label} content here...\n\nYou can use HTML for formatting:\n<h2>Heading</h2>\n<p>Paragraph text</p>\n<ul><li>List item</li></ul>`}
                                className="w-full px-4 py-3 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-primary-500 font-mono text-sm"
                            />
                            <p className="text-xs text-gray-500 dark:text-gray-400">
                                Use HTML tags for formatting. Click "Preview" to see how it will look.
                            </p>
                        </div>
                    )}
                </div>
            </div>
        </div>
    );
}
