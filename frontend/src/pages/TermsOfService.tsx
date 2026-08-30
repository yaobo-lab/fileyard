import { ArrowLeft, FileText, AlertTriangle, Scale, Code, Server, ShieldOff, CheckCircle } from 'lucide-react';
import { Link } from 'react-router-dom';
import { useGlobalSettings } from '../context/GlobalSettingsContext';

export default function TermsOfService() {
    const { settings } = useGlobalSettings();
    
    // If custom content is set, render it
    if (settings.tos_content && settings.tos_content.trim()) {
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
                                <FileText className="w-6 h-6 text-primary-600 dark:text-primary-400" />
                            </div>
                            <div>
                                <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Terms of Service</h1>
                            </div>
                        </div>
                    </div>
                    <div 
                        className="p-6 prose dark:prose-invert max-w-none"
                        dangerouslySetInnerHTML={{ __html: settings.tos_content }}
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
                            <FileText className="w-6 h-6 text-primary-600 dark:text-primary-400" />
                        </div>
                        <div>
                            <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Terms of Service</h1>
                            <p className="text-sm text-gray-500 dark:text-gray-400">Last updated: December 2024</p>
                        </div>
                    </div>
                </div>

                <div className="p-6 space-y-8">
                    {/* Introduction */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3">Agreement to Terms</h2>
                        <p className="text-gray-600 dark:text-gray-400 leading-relaxed">
                            By accessing or using ClovaLink, you agree to be bound by these Terms of Service. 
                            ClovaLink is open source software provided under the terms of its license agreement. 
                            Your use of this software is also subject to any additional terms set by your organization's administrator.
                        </p>
                    </section>

                    {/* Open Source License */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                            <Code className="w-5 h-5 text-gray-400" />
                            Open Source License
                        </h2>
                        <div className="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-4">
                            <p className="text-gray-600 dark:text-gray-400 leading-relaxed mb-3">
                                ClovaLink is released under an open source license. You are free to:
                            </p>
                            <ul className="text-gray-600 dark:text-gray-400 space-y-2">
                                <li className="flex items-start gap-2">
                                    <CheckCircle className="w-4 h-4 text-green-500 mt-0.5 flex-shrink-0" />
                                    <span>Use the software for any purpose, including commercial use</span>
                                </li>
                                <li className="flex items-start gap-2">
                                    <CheckCircle className="w-4 h-4 text-green-500 mt-0.5 flex-shrink-0" />
                                    <span>Modify the source code to suit your needs</span>
                                </li>
                                <li className="flex items-start gap-2">
                                    <CheckCircle className="w-4 h-4 text-green-500 mt-0.5 flex-shrink-0" />
                                    <span>Distribute copies of the original or modified software</span>
                                </li>
                                <li className="flex items-start gap-2">
                                    <CheckCircle className="w-4 h-4 text-green-500 mt-0.5 flex-shrink-0" />
                                    <span>Self-host on your own infrastructure</span>
                                </li>
                            </ul>
                            <p className="text-sm text-gray-500 dark:text-gray-400 mt-3">
                                Full license details are available in the project repository at{' '}
                                <a 
                                    href="https://github.com/clovalink/clovalink" 
                                    target="_blank" 
                                    rel="noopener noreferrer"
                                    className="text-primary-600 dark:text-primary-400 hover:underline"
                                >
                                    github.com/clovalink/clovalink
                                </a>
                            </p>
                        </div>
                    </section>

                    {/* Disclaimer of Warranties */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                            <ShieldOff className="w-5 h-5 text-gray-400" />
                            Disclaimer of Warranties
                        </h2>
                        <div className="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg p-4">
                            <div className="flex items-start gap-3">
                                <AlertTriangle className="w-5 h-5 text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" />
                                <div className="text-gray-700 dark:text-gray-300">
                                    <p className="font-medium mb-2">THIS SOFTWARE IS PROVIDED "AS IS"</p>
                                    <p className="text-sm leading-relaxed">
                                        THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, 
                                        INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR 
                                        PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS, COPYRIGHT HOLDERS, OR 
                                        CONTRIBUTORS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION 
                                        OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE 
                                        OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
                                    </p>
                                </div>
                            </div>
                        </div>
                    </section>

                    {/* Limitation of Liability */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                            <Scale className="w-5 h-5 text-gray-400" />
                            Limitation of Liability
                        </h2>
                        <div className="text-gray-600 dark:text-gray-400 space-y-3 leading-relaxed">
                            <p>
                                To the maximum extent permitted by applicable law, in no event shall ClovaLink, 
                                its authors, contributors, or affiliated organizations be liable for:
                            </p>
                            <ul className="list-disc list-inside space-y-1 ml-4">
                                <li>Any indirect, incidental, special, consequential, or punitive damages</li>
                                <li>Loss of profits, data, use, goodwill, or other intangible losses</li>
                                <li>Any damages resulting from unauthorized access to or alteration of your data</li>
                                <li>Any interruption or cessation of functionality</li>
                                <li>Any bugs, viruses, or other harmful code transmitted through the software</li>
                                <li>Any errors or omissions in any content</li>
                            </ul>
                            <p>
                                This limitation applies regardless of the legal theory under which such damages are sought.
                            </p>
                        </div>
                    </section>

                    {/* User Responsibilities */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3 flex items-center gap-2">
                            <Server className="w-5 h-5 text-gray-400" />
                            User Responsibilities
                        </h2>
                        <p className="text-gray-600 dark:text-gray-400 leading-relaxed mb-3">
                            As a user of ClovaLink, you agree to:
                        </p>
                        <ul className="text-gray-600 dark:text-gray-400 space-y-2 list-disc list-inside">
                            <li>Maintain the confidentiality of your account credentials</li>
                            <li>Use the software in compliance with applicable laws and regulations</li>
                            <li>Not attempt to circumvent security measures or access controls</li>
                            <li>Not upload malicious content or engage in harmful activities</li>
                            <li>Respect the intellectual property rights of others</li>
                            <li>Comply with your organization's acceptable use policies</li>
                        </ul>
                    </section>

                    {/* Compliance Features */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3">Compliance Features Disclaimer</h2>
                        <div className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4">
                            <p className="text-gray-700 dark:text-gray-300 text-sm leading-relaxed">
                                ClovaLink provides features designed to assist with regulatory compliance (HIPAA, SOX, GDPR, etc.). 
                                However, <strong>use of these features does not guarantee compliance</strong> with any specific regulation. 
                                Compliance depends on proper configuration, organizational policies, and adherence to applicable requirements. 
                                Consult with qualified legal and compliance professionals to ensure your use meets regulatory requirements.
                            </p>
                        </div>
                    </section>

                    {/* Data Responsibility */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3">Data Responsibility</h2>
                        <p className="text-gray-600 dark:text-gray-400 leading-relaxed">
                            For self-hosted installations, the organization hosting the software is solely responsible for:
                        </p>
                        <ul className="text-gray-600 dark:text-gray-400 space-y-2 list-disc list-inside mt-3">
                            <li>Data backup and recovery procedures</li>
                            <li>Security configuration and maintenance</li>
                            <li>Compliance with data protection regulations</li>
                            <li>Access control and user management</li>
                            <li>Infrastructure security and updates</li>
                        </ul>
                    </section>

                    {/* Modifications */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3">Modifications to Terms</h2>
                        <p className="text-gray-600 dark:text-gray-400 leading-relaxed">
                            These terms may be updated periodically. Continued use of ClovaLink after changes constitutes 
                            acceptance of the modified terms. Check this page regularly for updates.
                        </p>
                    </section>

                    {/* Governing Law */}
                    <section>
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-3">Governing Law</h2>
                        <p className="text-gray-600 dark:text-gray-400 leading-relaxed">
                            These terms shall be governed by and construed in accordance with the laws of the jurisdiction 
                            in which you operate, without regard to its conflict of law provisions.
                        </p>
                    </section>

                    {/* Contact */}
                    <section className="border-t border-gray-200 dark:border-gray-700 pt-6">
                        <p className="text-sm text-gray-500 dark:text-gray-400">
                            For questions about these terms regarding this instance, contact your organization's administrator. 
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
