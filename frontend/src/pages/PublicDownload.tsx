import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Download, FileText, Shield, AlertCircle, Loader2, Lock, Clock, Users, Archive, Globe } from 'lucide-react';
import { FileSystemFolderGlyph, FileGenericPaper } from '../components/FileGlyphs';
import clsx from 'clsx';
import { useAuth } from '../context/AuthContext';
import { useI18n, useTranslations } from '../context/I18nContext';

interface ShareInfo {
    file_name: string;
    size_bytes: number;
    size_formatted: string;
    content_type: string | null;
    is_public: boolean;
    is_directory: boolean;
    expires_at: string | null;
    download_count: number;
    shared_by: string;
}

export function PublicDownload() {
    const { token } = useParams<{ token: string }>();
    const navigate = useNavigate();
    const { isAuthenticated } = useAuth();
    const { locale, setLocale } = useI18n();
    const t = useTranslations('ShareDownload');
    
    const [shareInfo, setShareInfo] = useState<ShareInfo | null>(null);
    const [loading, setLoading] = useState(true);
    const [errorCode, setErrorCode] = useState<string | null>(null);
    const [downloading, setDownloading] = useState(false);
    const [downloadComplete, setDownloadComplete] = useState(false);

    useEffect(() => {
        fetchShareInfo();
    }, [token]);

    const fetchShareInfo = async () => {
        try {
            const API_URL = import.meta.env.VITE_API_URL || '';
            const response = await fetch(`${API_URL}/api/share/${token}/info`);
            
            if (response.ok) {
                const data = await response.json();
                setShareInfo(data);
                setErrorCode(null);
            } else if (response.status === 404) {
                setErrorCode('errInvalid');
            } else if (response.status === 410) {
                setErrorCode('errExpired');
            } else {
                setErrorCode('errLoadFailed');
            }
        } catch (err) {
            setErrorCode('errNetwork');
        } finally {
            setLoading(false);
        }
    };

    const handleDownload = async () => {
        if (!shareInfo) return;

        // If not public and not authenticated, redirect to login
        if (!shareInfo.is_public && !isAuthenticated) {
            // Store the current URL to redirect back after login
            sessionStorage.setItem('redirect_after_login', window.location.pathname);
            navigate('/login');
            return;
        }

        setDownloading(true);
        
        try {
            const API_URL = import.meta.env.VITE_API_URL || '';
            const headers: Record<string, string> = {};
            
            // Add auth header if logged in (for private shares)
            if (!shareInfo.is_public) {
                const authToken = localStorage.getItem('auth_token');
                if (authToken) {
                    headers['Authorization'] = `Bearer ${authToken}`;
                }
            }

            const response = await fetch(`${API_URL}/api/share/${token}`, { headers });
            
            if (response.ok) {
                const blob = await response.blob();
                const url = window.URL.createObjectURL(blob);
                const a = document.createElement('a');
                a.href = url;
                a.download = shareInfo.file_name;
                document.body.appendChild(a);
                a.click();
                window.URL.revokeObjectURL(url);
                document.body.removeChild(a);
                setDownloadComplete(true);
            } else if (response.status === 401) {
                // Need to login
                sessionStorage.setItem('redirect_after_login', window.location.pathname);
                navigate('/login');
            } else if (response.status === 403) {
                setErrorCode('errNoPermission');
            } else if (response.status === 410) {
                setErrorCode('errExpired');
            } else {
                setErrorCode('errDownloadFailed');
            }
        } catch (err) {
            setErrorCode('errCheckConnection');
        } finally {
            setDownloading(false);
        }
    };

    const getFileIcon = () => {
        // Folder icon for directories
        if (shareInfo?.is_directory) {
            return <FileSystemFolderGlyph size="lg" className="h-16 w-auto" />;
        }
        
        if (shareInfo?.file_name) {
            return <FileGenericPaper fileName={shareInfo.file_name} className="w-16 h-20" size="lg" />;
        }
        
        return <FileText className="w-16 h-16 text-gray-400" />;
    };

    const formatExpirationDate = (dateStr: string) => {
        const date = new Date(dateStr);
        return date.toLocaleDateString(locale === 'zh' ? 'zh-CN' : 'en-US', {
            year: 'numeric',
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
        });
    };

    return (
        <div className="min-h-screen bg-white flex flex-col items-center justify-center p-4 relative overflow-hidden">
            {/* Language Switcher */}
            <div className="absolute top-4 right-4 z-20">
                <button
                    onClick={() => setLocale(locale === 'zh' ? 'en' : 'zh')}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-gray-700 hover:text-gray-900 bg-white/80 hover:bg-white border border-gray-200 rounded-lg shadow-xs backdrop-blur-xs transition-colors"
                >
                    <Globe className="w-3.5 h-3.5 text-gray-500" />
                    <span>{locale === 'zh' ? 'English' : '简体中文'}</span>
                </button>
            </div>

            {/* Background decoration */}
            <div className="absolute top-0 left-0 w-full h-full overflow-hidden z-0 pointer-events-none">
                <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-primary-600/20 rounded-full blur-3xl"></div>
                <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-blue-600/20 rounded-full blur-3xl"></div>
            </div>

            <div className="w-full max-w-md z-10">
                <div className="text-center mb-8">
                    <h1 className="text-3xl font-bold text-gray-900 tracking-tight">{t('title')}</h1>
                    <p className="text-gray-500 mt-2">{t('subtitle')}</p>
                </div>

                <div className="bg-white border border-gray-200 rounded-2xl shadow-xl ring-1 ring-gray-900/5 overflow-hidden">
                    <div className="p-8">
                        {loading ? (
                            <div className="flex flex-col items-center justify-center py-8">
                                <Loader2 className="w-10 h-10 text-primary-600 animate-spin mb-4" />
                                <p className="text-gray-500">{t('loading')}</p>
                            </div>
                        ) : errorCode ? (
                            <div className="text-center py-8">
                                <div className="mx-auto flex items-center justify-center h-20 w-20 rounded-full bg-red-100 mb-6">
                                    <AlertCircle className="h-10 w-10 text-red-500" />
                                </div>
                                <h3 className="text-xl font-bold text-gray-900 mb-2">{t('unableToAccess')}</h3>
                                <p className="text-gray-500 mb-6">{t(errorCode)}</p>
                                <a
                                    href="/"
                                    className="inline-flex items-center px-4 py-2 text-sm font-medium text-primary-600 hover:text-primary-700"
                                >
                                    {t('goHome')}
                                </a>
                            </div>
                        ) : downloadComplete ? (
                            <div className="text-center py-8">
                                <div className="mx-auto flex items-center justify-center h-20 w-20 rounded-full bg-gradient-to-br from-green-400 to-green-600 mb-6">
                                    <Download className="h-10 w-10 text-white" />
                                </div>
                                <h3 className="text-2xl font-bold text-gray-900 mb-2">{t('downloadComplete')}</h3>
                                <p className="text-gray-500 mb-2">
                                    {t('downloadedMsg', { name: shareInfo?.file_name || '' })}
                                </p>
                                <div className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-gray-50 border border-gray-200 text-sm text-gray-500 mb-8">
                                    <Shield className="w-4 h-4 text-green-500" />
                                    <span>{t('secureTransferComplete')}</span>
                                </div>
                                <button
                                    onClick={() => setDownloadComplete(false)}
                                    className="w-full py-3 px-4 bg-gradient-to-r from-primary-600 to-primary-700 text-white rounded-xl font-semibold hover:from-primary-700 hover:to-primary-800 transition-all duration-200 shadow-lg shadow-primary-500/50"
                                >
                                    {t('downloadAgain')}
                                </button>
                            </div>
                        ) : shareInfo ? (
                            <div className="text-center">
                                {/* File icon and info */}
                                <div className="mb-6">
                                    <div className="mx-auto flex items-center justify-center h-24 w-24 rounded-2xl bg-gray-50 border border-gray-200 mb-4">
                                        {getFileIcon()}
                                    </div>
                                    <h3 className="text-lg font-semibold text-gray-900 mb-1 break-all px-4">
                                        {shareInfo.file_name}
                                    </h3>
                                    <p className="text-sm text-gray-500">{shareInfo.size_formatted}</p>
                                </div>

                                {/* Share info badges */}
                                <div className="flex flex-wrap justify-center gap-2 mb-6">
                                    {shareInfo.is_directory && (
                                        <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-purple-50 text-purple-700 text-xs font-medium">
                                            <Archive className="w-3 h-3" />
                                            {t('folderZip')}
                                        </span>
                                    )}
                                    {shareInfo.is_public ? (
                                        <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-green-50 text-green-700 text-xs font-medium">
                                            <Users className="w-3 h-3" />
                                            {t('publicLink')}
                                        </span>
                                    ) : (
                                        <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-blue-50 text-blue-700 text-xs font-medium">
                                            <Lock className="w-3 h-3" />
                                            {t('orgOnly')}
                                        </span>
                                    )}
                                    {shareInfo.expires_at && (
                                        <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-amber-50 text-amber-700 text-xs font-medium">
                                            <Clock className="w-3 h-3" />
                                            {t('expiresAt', { date: formatExpirationDate(shareInfo.expires_at) })}
                                        </span>
                                    )}
                                </div>

                                {/* Login notice for private shares */}
                                {!shareInfo.is_public && !isAuthenticated && (
                                    <div className="mb-6 p-3 bg-blue-50 border border-blue-200 rounded-lg text-sm text-blue-700">
                                        <Lock className="w-4 h-4 inline mr-2" />
                                        {t('loginRequired')}
                                    </div>
                                )}

                                {/* Download button */}
                                <button
                                    onClick={handleDownload}
                                    disabled={downloading}
                                    className={clsx(
                                        "w-full py-3 px-4 rounded-xl font-semibold transition-all duration-200 flex items-center justify-center gap-2",
                                        downloading
                                            ? "bg-gray-200 text-gray-500 cursor-not-allowed"
                                            : "bg-gradient-to-r from-primary-600 to-primary-700 text-white hover:from-primary-700 hover:to-primary-800 shadow-lg shadow-primary-500/50"
                                    )}
                                >
                                    {downloading ? (
                                        <>
                                            <Loader2 className="w-5 h-5 animate-spin" />
                                            {t('downloading')}
                                        </>
                                    ) : !shareInfo.is_public && !isAuthenticated ? (
                                        <>
                                            <Lock className="w-5 h-5" />
                                            {t('loginToDownload')}
                                        </>
                                    ) : (
                                        <>
                                            <Download className="w-5 h-5" />
                                            {shareInfo.is_directory ? t('downloadFolder') : t('downloadFile')}
                                        </>
                                    )}
                                </button>

                                {/* Shared by */}
                                <p className="mt-4 text-xs text-gray-400">
                                    {t('sharedBy', { name: shareInfo.shared_by })}
                                </p>
                            </div>
                        ) : null}
                    </div>

                    <div className="bg-gray-50 px-8 py-4 border-t border-gray-200 flex items-center justify-center text-xs text-gray-500">
                        <Shield className="w-3 h-3 mr-1.5" />
                        <span>{t('sslSecure')}</span>
                    </div>
                </div>
            </div>
        </div>
    );
}

