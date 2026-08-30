import { useState, useEffect } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Download, FileText, Shield, AlertCircle, Loader2, Lock, Clock, Users, Folder, Archive } from 'lucide-react';
import clsx from 'clsx';
import { Logo } from '../components/Logo';
import { useAuth } from '../context/AuthContext';

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
    
    const [shareInfo, setShareInfo] = useState<ShareInfo | null>(null);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
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
            } else if (response.status === 404) {
                setError('This share link is invalid or has been revoked.');
            } else if (response.status === 410) {
                setError('This share link has expired.');
            } else {
                setError('Failed to load share information.');
            }
        } catch (err) {
            setError('Unable to connect to server.');
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
                setError('You do not have permission to download this file.');
            } else if (response.status === 410) {
                setError('This share link has expired.');
            } else {
                setError('Download failed. Please try again.');
            }
        } catch (err) {
            setError('Download failed. Please check your connection.');
        } finally {
            setDownloading(false);
        }
    };

    const getFileIcon = () => {
        // Folder icon for directories
        if (shareInfo?.is_directory) {
            return <Folder className="w-16 h-16 text-yellow-500" />;
        }
        
        if (!shareInfo?.content_type) return <FileText className="w-16 h-16 text-gray-400" />;
        
        const type = shareInfo.content_type;
        if (type.startsWith('image/')) {
            return <FileText className="w-16 h-16 text-purple-500" />;
        } else if (type === 'application/pdf') {
            return <FileText className="w-16 h-16 text-red-500" />;
        } else if (type.includes('word') || type.includes('document')) {
            return <FileText className="w-16 h-16 text-blue-500" />;
        } else if (type.includes('sheet') || type.includes('excel')) {
            return <FileText className="w-16 h-16 text-green-500" />;
        } else if (type.startsWith('video/')) {
            return <FileText className="w-16 h-16 text-pink-500" />;
        } else if (type.startsWith('audio/')) {
            return <FileText className="w-16 h-16 text-orange-500" />;
        }
        return <FileText className="w-16 h-16 text-gray-400" />;
    };

    const formatExpirationDate = (dateStr: string) => {
        const date = new Date(dateStr);
        return date.toLocaleDateString(undefined, {
            year: 'numeric',
            month: 'short',
            day: 'numeric',
            hour: '2-digit',
            minute: '2-digit',
        });
    };

    return (
        <div className="min-h-screen bg-white flex flex-col items-center justify-center p-4 relative overflow-hidden">
            {/* Background decoration */}
            <div className="absolute top-0 left-0 w-full h-full overflow-hidden z-0 pointer-events-none">
                <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-primary-600/20 rounded-full blur-3xl"></div>
                <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-blue-600/20 rounded-full blur-3xl"></div>
            </div>

            <div className="w-full max-w-md z-10">
                <div className="text-center mb-8">
                    <div className="flex justify-center mb-4">
                        <div className="h-60 w-auto text-primary-600">
                            <Logo className="h-60 w-auto text-primary-600" />
                        </div>
                    </div>
                    <h1 className="text-3xl font-bold text-gray-900 tracking-tight">Secure File Download</h1>
                    <p className="text-gray-500 mt-2">A file has been shared with you.</p>
                </div>

                <div className="bg-white border border-gray-200 rounded-2xl shadow-xl ring-1 ring-gray-900/5 overflow-hidden">
                    <div className="p-8">
                        {loading ? (
                            <div className="flex flex-col items-center justify-center py-8">
                                <Loader2 className="w-10 h-10 text-primary-600 animate-spin mb-4" />
                                <p className="text-gray-500">Loading share information...</p>
                            </div>
                        ) : error ? (
                            <div className="text-center py-8">
                                <div className="mx-auto flex items-center justify-center h-20 w-20 rounded-full bg-red-100 mb-6">
                                    <AlertCircle className="h-10 w-10 text-red-500" />
                                </div>
                                <h3 className="text-xl font-bold text-gray-900 mb-2">Unable to Access File</h3>
                                <p className="text-gray-500 mb-6">{error}</p>
                                <a
                                    href="/"
                                    className="inline-flex items-center px-4 py-2 text-sm font-medium text-primary-600 hover:text-primary-700"
                                >
                                    Go to Homepage
                                </a>
                            </div>
                        ) : downloadComplete ? (
                            <div className="text-center py-8">
                                <div className="mx-auto flex items-center justify-center h-20 w-20 rounded-full bg-gradient-to-br from-green-400 to-green-600 mb-6">
                                    <Download className="h-10 w-10 text-white" />
                                </div>
                                <h3 className="text-2xl font-bold text-gray-900 mb-2">Download Complete!</h3>
                                <p className="text-gray-500 mb-2">
                                    <span className="font-semibold text-gray-900">{shareInfo?.file_name}</span> has been downloaded.
                                </p>
                                <div className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-gray-50 border border-gray-200 text-sm text-gray-500 mb-8">
                                    <Shield className="w-4 h-4 text-green-500" />
                                    <span>Secure transfer complete</span>
                                </div>
                                <button
                                    onClick={() => setDownloadComplete(false)}
                                    className="w-full py-3 px-4 bg-gradient-to-r from-primary-600 to-primary-700 text-white rounded-xl font-semibold hover:from-primary-700 hover:to-primary-800 transition-all duration-200 shadow-lg shadow-primary-500/50"
                                >
                                    Download Again
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
                                            Folder (ZIP)
                                        </span>
                                    )}
                                    {shareInfo.is_public ? (
                                        <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-green-50 text-green-700 text-xs font-medium">
                                            <Users className="w-3 h-3" />
                                            Public Link
                                        </span>
                                    ) : (
                                        <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-blue-50 text-blue-700 text-xs font-medium">
                                            <Lock className="w-3 h-3" />
                                            Organization Only
                                        </span>
                                    )}
                                    {shareInfo.expires_at && (
                                        <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full bg-amber-50 text-amber-700 text-xs font-medium">
                                            <Clock className="w-3 h-3" />
                                            Expires {formatExpirationDate(shareInfo.expires_at)}
                                        </span>
                                    )}
                                </div>

                                {/* Login notice for private shares */}
                                {!shareInfo.is_public && !isAuthenticated && (
                                    <div className="mb-6 p-3 bg-blue-50 border border-blue-200 rounded-lg text-sm text-blue-700">
                                        <Lock className="w-4 h-4 inline mr-2" />
                                        You'll need to log in to download this file.
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
                                            Downloading...
                                        </>
                                    ) : !shareInfo.is_public && !isAuthenticated ? (
                                        <>
                                            <Lock className="w-5 h-5" />
                                            Log in to Download
                                        </>
                                    ) : (
                                        <>
                                            <Download className="w-5 h-5" />
                                            {shareInfo.is_directory ? 'Download Folder (ZIP)' : 'Download File'}
                                        </>
                                    )}
                                </button>

                                {/* Shared by */}
                                <p className="mt-4 text-xs text-gray-400">
                                    Shared by {shareInfo.shared_by}
                                </p>
                            </div>
                        ) : null}
                    </div>

                    <div className="bg-gray-50 px-8 py-4 border-t border-gray-200 flex items-center justify-center text-xs text-gray-500">
                        <Shield className="w-3 h-3 mr-1.5" />
                        <span>256-bit SSL Secure Transfer</span>
                    </div>
                </div>

                <p className="mt-8 text-center text-xs text-gray-500">
                    &copy; {new Date().getFullYear()} ClovaLink. All rights reserved.
                </p>
            </div>
        </div>
    );
}

