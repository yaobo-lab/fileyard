import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import {
    Share2,
    Download,
    Eye,
    Trash2,
    Copy,
    Check,
    Globe,
    User,
    Lock,
    Clock,
    Loader2,
    Search,
    ChevronLeft,
    ChevronRight,
    ExternalLink,
} from 'lucide-react';
import clsx from 'clsx';
import { useAuthFetch } from '../context/AuthContext';
import { useGlobalSettings } from '../context/GlobalSettingsContext';
import { useTranslations } from '../context/I18nContext';
import { useModalDialog } from '../context/ModalDialogContext';
import { FilePreviewModal, detectFileKind } from '../components/FilePreviewModal';
import { FileGlyphVisual } from '../components/FileGlyphs';
import { copyToClipboard } from '@/lib/utils';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';

interface MyShareItem {
    id: string;
    file_id: string;
    name: string;
    size: number;
    content_type: string | null;
    folder_path: string | null;
    token: string;
    is_public: boolean;
    is_directory: boolean;
    expires_at: string | null;
    download_count: number;
    created_at: string;
    shared_with_user_id: string | null;
    shared_with_user_name: string | null;
    shared_with_user_email: string | null;
}

const getShareDaysText = (createdAt: string, expiresAt: string | null): string => {
    if (!expiresAt) return '永久有效';
    const created = new Date(createdAt).getTime();
    const expires = new Date(expiresAt).getTime();
    const diffMs = expires - created;
    const days = Math.round(diffMs / (1000 * 60 * 60 * 24));
    if (days <= 0) return '1 天';
    return `${days} 天`;
};

const getFileType = (contentType: string | null, name: string): 'image' | 'document' | 'video' | 'audio' | 'folder' => {
    if (!contentType) {
        const ext = name.split('.').pop()?.toLowerCase();
        if (['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp'].includes(ext || '')) return 'image';
        if (['mp4', 'mov', 'avi', 'webm', 'mkv'].includes(ext || '')) return 'video';
        if (['mp3', 'wav', 'flac', 'aac', 'ogg', 'm4a'].includes(ext || '')) return 'audio';
        return 'document';
    }
    if (contentType.startsWith('image/')) return 'image';
    if (contentType.startsWith('video/')) return 'video';
    if (contentType.startsWith('audio/')) return 'audio';
    return 'document';
};

const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

export function MyShares() {
    const t = useTranslations('MyShares');
    const authFetch = useAuthFetch();
    const { formatDate } = useGlobalSettings();
    const { confirm: modalConfirm, alert: modalAlert } = useModalDialog();
    const navigate = useNavigate();

    const [shares, setShares] = useState<MyShareItem[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);
    const [searchTerm, setSearchTerm] = useState('');
    const [page, setPage] = useState(1);
    const [totalPages, setTotalPages] = useState(1);
    const [total, setTotal] = useState(0);
    const perPage = 20;

    const [copiedToken, setCopiedToken] = useState<string | null>(null);
    const [revokingId, setRevokingId] = useState<string | null>(null);

    // Preview modal state
    const [previewFile, setPreviewFile] = useState<{ name: string; url: string; type: 'image' | 'document' | 'video' | 'audio' | 'folder' } | null>(null);
    const [isPreviewOpen, setIsPreviewOpen] = useState(false);

    const fetchMyShares = useCallback(async () => {
        setLoading(true);
        setError(null);
        try {
            const res = await authFetch(`/api/my-shares?page=${page}&per_page=${perPage}`);
            if (res.ok) {
                const data = await res.json();
                setShares(data.shares || []);
                setTotalPages(data.total_pages || 1);
                setTotal(data.total || 0);
            } else {
                setError('Failed to load my shares');
            }
        } catch {
            setError('Failed to load my shares');
        } finally {
            setLoading(false);
        }
    }, [authFetch, page, perPage]);

    useEffect(() => {
        fetchMyShares();
    }, [fetchMyShares]);

    const handleCopyLink = async (token: string, e?: React.MouseEvent) => {
        e?.stopPropagation();
        const shareLink = `${window.location.origin}/share/${token}`;
        const success = await copyToClipboard(shareLink);
        if (success) {
            setCopiedToken(token);
            setTimeout(() => setCopiedToken(null), 2500);
        }
    };

    const handleRevokeShare = async (share: MyShareItem, e?: React.MouseEvent) => {
        e?.stopPropagation();
        const ok = await modalConfirm({
            title: t('revokeShare') || '取消分享',
            description: t('revokeConfirm') || `确定要取消 "${share.name}" 的分享吗？取消后该链接或接收人将无法继续访问。`,
            variant: 'destructive',
            confirmText: '确认取消',
            cancelText: '暂不取消',
        });
        if (!ok) return;

        setRevokingId(share.id);
        try {
            const res = await authFetch(`/api/my-shares/${share.id}`, {
                method: 'DELETE',
            });
            if (res.ok) {
                fetchMyShares();
            } else {
                await modalAlert({
                    title: '操作失败',
                    description: t('revokeFailed') || '取消分享失败，请稍后重试',
                    variant: 'destructive',
                });
            }
        } catch {
            await modalAlert({
                title: '操作失败',
                description: t('revokeFailed') || '取消分享失败，请稍后重试',
                variant: 'destructive',
            });
        } finally {
            setRevokingId(null);
        }
    };

    const handlePreview = (share: MyShareItem, e?: React.MouseEvent) => {
        e?.stopPropagation();
        const fileType = getFileType(share.content_type, share.name);
        setPreviewFile({
            name: share.name,
            url: `/api/share/${share.token}`,
            type: fileType,
        });
        setIsPreviewOpen(true);
    };

    // Filter shares by search term
    const filteredShares = useMemo(() => {
        if (!searchTerm.trim()) return shares;
        const term = searchTerm.toLowerCase().trim();
        return shares.filter(
            (s) =>
                s.name.toLowerCase().includes(term) ||
                (s.shared_with_user_name && s.shared_with_user_name.toLowerCase().includes(term)) ||
                (s.shared_with_user_email && s.shared_with_user_email.toLowerCase().includes(term))
        );
    }, [shares, searchTerm]);

    const isExpired = (expiresAt: string | null) => {
        if (!expiresAt) return false;
        return new Date(expiresAt).getTime() < Date.now();
    };

    return (
        <div className="min-h-screen bg-gray-50 dark:bg-gray-900 pb-12">
            {/* Header */}
            <div className="bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700">
                <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
                    <div className="flex items-center justify-between">
                        <div className="flex items-center gap-4">
                            <div className="p-3 bg-primary-100 dark:bg-primary-900/30 rounded-xl">
                                <Share2 className="w-6 h-6 text-primary-600 dark:text-primary-400" />
                            </div>
                            <div>
                                <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
                                    {t('title') || '我的分享'}
                                </h1>
                                <p className="text-sm text-gray-500 dark:text-gray-400">
                                    {t('description') || '查看和管理您分享给他人或创建的公开分享链接'}
                                </p>
                            </div>
                        </div>
                        <Button
                            variant="outline"
                            onClick={() => navigate('/files')}
                            className="gap-2"
                        >
                            全部文件
                        </Button>
                    </div>
                </div>
            </div>

            <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6 space-y-4">
                {/* Search Toolbar */}
                <div className="flex items-center justify-between gap-4">
                    <div className="relative w-72 sm:w-80">
                        <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                        <input
                            type="text"
                            placeholder={t('searchPlaceholder') || '搜索文件名或分享对象...'}
                            value={searchTerm}
                            onChange={(e) => setSearchTerm(e.target.value)}
                            className="w-full pl-9 pr-3 py-1.5 text-xs border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-1 focus:ring-primary-500 bg-white dark:bg-gray-800 text-gray-900 dark:text-white shadow-sm"
                        />
                    </div>
                    <div className="text-xs text-gray-500 dark:text-gray-400">
                        共 {total} 条分享记录
                    </div>
                </div>

                {/* Loading State */}
                {loading && (
                    <div className="flex items-center justify-center py-16">
                        <Loader2 className="w-8 h-8 text-primary-600 animate-spin" />
                    </div>
                )}

                {/* Error State */}
                {error && (
                    <div className="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-red-700 dark:text-red-300 text-sm">
                        {error}
                    </div>
                )}

                {/* Empty State */}
                {!loading && !error && filteredShares.length === 0 && (
                    <div className="text-center py-16 bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700">
                        <Share2 className="w-12 h-12 mx-auto text-gray-400 mb-4 opacity-60" />
                        <h3 className="text-base font-medium text-gray-900 dark:text-white mb-1">
                            {t('noShares') || '暂无分享记录'}
                        </h3>
                        <p className="text-xs text-gray-500 dark:text-gray-400 max-w-sm mx-auto">
                            {t('noSharesDesc') || '当您在文件列表中分享文件后，分享记录将显示在此处。'}
                        </p>
                    </div>
                )}

                {/* Table */}
                {!loading && filteredShares.length > 0 && (
                    <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
                        <div className="hidden md:grid grid-cols-12 gap-4 px-6 py-3 bg-gray-50 dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                            <div className="col-span-4">{t('colName') || '文件名'}</div>
                            <div className="col-span-2">{t('colShareType') || '分享形式'}</div>
                            <div className="col-span-2">{t('colDuration') || '过期天数'}</div>
                            <div className="col-span-1">{t('colDownloads') || '访问次数'}</div>
                            <div className="col-span-1">{t('colExpires') || '有效期'}</div>
                            <div className="col-span-2 text-right">{t('colActions') || '操作'}</div>
                        </div>

                        <div className="divide-y divide-gray-100 dark:divide-gray-700">
                            {filteredShares.map((share) => {
                                const expired = isExpired(share.expires_at);
                                const fileType = getFileType(share.content_type, share.name);
                                const durationText = getShareDaysText(share.created_at, share.expires_at);

                                return (
                                    <div
                                        key={share.id}
                                        className="flex flex-col md:grid md:grid-cols-12 gap-3 md:gap-4 px-6 py-3.5 hover:bg-gray-50 dark:hover:bg-gray-700/30 transition-colors items-center"
                                    >
                                        {/* File Info */}
                                        <div className="col-span-4 flex items-center gap-3 w-full min-w-0">
                                            <div className="flex-shrink-0 w-8 h-8 flex items-center justify-center">
                                                <FileGlyphVisual file={{ id: share.file_id, name: share.name, type: fileType }} size="sm" />
                                            </div>
                                            <div className="min-w-0 flex-1">
                                                <p className="text-xs font-medium text-gray-900 dark:text-white truncate" title={share.name}>
                                                    {share.name}
                                                </p>
                                                <p className="text-[11px] text-gray-500 dark:text-gray-400 truncate">
                                                    {formatBytes(share.size)}
                                                    {share.folder_path ? ` · ${share.folder_path}` : ''}
                                                </p>
                                            </div>
                                        </div>

                                        {/* Share Type */}
                                        <div className="col-span-2 flex items-center">
                                            {share.is_public ? (
                                                <Badge variant="outline" className="text-[11px] gap-1 text-blue-600 bg-blue-50 dark:bg-blue-950/40 border-blue-200 dark:border-blue-900 font-normal">
                                                    <Globe className="w-3 h-3" />
                                                    {t('publicLink') || '获得链接的人'}
                                                </Badge>
                                            ) : (
                                                <Badge variant="outline" className="text-[11px] gap-1 text-gray-800 dark:text-gray-200 bg-gray-100 dark:bg-gray-800 border-gray-300 dark:border-gray-600 font-medium">
                                                    <Lock className="w-3 h-3" />
                                                    {t('orgOnly') || '仅企业成员'}
                                                </Badge>
                                            )}
                                        </div>

                                        {/* Expiration Days */}
                                        <div className="col-span-2 flex items-center text-xs text-gray-700 dark:text-gray-300">
                                            <Clock className="w-3.5 h-3.5 text-gray-400 mr-1.5 flex-shrink-0" />
                                            <span className="font-medium text-gray-900 dark:text-white">
                                                {durationText}
                                            </span>
                                            {expired && (
                                                <Badge variant="destructive" className="ml-2 text-[10px] px-1 py-0 font-normal">
                                                    已过期
                                                </Badge>
                                            )}
                                        </div>

                                        {/* Downloads / Hits */}
                                        <div className="col-span-1 flex items-center text-xs text-gray-600 dark:text-gray-300">
                                            <span className="font-mono">{share.download_count}</span>
                                            <span className="text-gray-400 ml-1 text-[11px]">次</span>
                                        </div>

                                        {/* Expiry Date */}
                                        <div className="col-span-1 flex items-center text-xs">
                                            {share.expires_at ? (
                                                <span className="text-gray-500 dark:text-gray-400 text-[11px]">
                                                    {new Date(share.expires_at).toLocaleDateString()}
                                                </span>
                                            ) : (
                                                <span className="text-gray-400 text-[11px]">永久</span>
                                            )}
                                        </div>

                                        {/* Actions */}
                                        <div className="col-span-2 flex items-center justify-end gap-1">
                                            {share.is_public && (
                                                <Button
                                                    variant="ghost"
                                                    size="sm"
                                                    onClick={(e) => handleCopyLink(share.token, e)}
                                                    className="h-7 px-2 text-xs gap-1 text-primary-600 hover:text-primary-700"
                                                    title={t('copyLink') || '复制分享链接'}
                                                >
                                                    {copiedToken === share.token ? (
                                                        <>
                                                            <Check className="w-3.5 h-3.5 text-green-600" />
                                                            <span className="text-green-600">已复制</span>
                                                        </>
                                                    ) : (
                                                        <>
                                                            <Copy className="w-3.5 h-3.5" />
                                                            <span>链接</span>
                                                        </>
                                                    )}
                                                </Button>
                                            )}

                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                onClick={(e) => handlePreview(share, e)}
                                                className="h-7 w-7 p-0 text-gray-500 hover:text-gray-700"
                                                title={t('preview') || '预览文件'}
                                            >
                                                <Eye className="w-3.5 h-3.5" />
                                            </Button>

                                            <Button
                                                variant="ghost"
                                                size="sm"
                                                onClick={(e) => handleRevokeShare(share, e)}
                                                disabled={revokingId === share.id}
                                                className="h-7 px-2 text-xs gap-1 text-red-600 hover:text-red-700 hover:bg-red-50 dark:hover:bg-red-950/30"
                                                title={t('revokeShare') || '取消分享'}
                                            >
                                                <Trash2 className="w-3.5 h-3.5" />
                                                <span>取消</span>
                                            </Button>
                                        </div>
                                    </div>
                                );
                            })}
                        </div>

                        {/* Pagination */}
                        {totalPages > 1 && (
                            <div className="flex items-center justify-between px-6 py-3 bg-gray-50 dark:bg-gray-800 border-t border-gray-200 dark:border-gray-700 text-xs">
                                <span className="text-gray-500">
                                    第 {page} / {totalPages} 页
                                </span>
                                <div className="flex gap-2">
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        disabled={page <= 1}
                                        onClick={() => setPage((p) => Math.max(1, p - 1))}
                                        className="h-7 text-xs"
                                    >
                                        <ChevronLeft className="w-3.5 h-3.5 mr-1" />
                                        上一页
                                    </Button>
                                    <Button
                                        variant="outline"
                                        size="sm"
                                        disabled={page >= totalPages}
                                        onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                                        className="h-7 text-xs"
                                    >
                                        下一页
                                        <ChevronRight className="w-3.5 h-3.5 ml-1" />
                                    </Button>
                                </div>
                            </div>
                        )}
                    </div>
                )}
            </div>

            {/* Preview Modal */}
            {previewFile && (
                <FilePreviewModal
                    isOpen={isPreviewOpen}
                    onClose={() => {
                        setIsPreviewOpen(false);
                        setPreviewFile(null);
                    }}
                    file={{
                        id: '',
                        name: previewFile.name,
                        url: previewFile.url,
                        type: previewFile.type,
                    }}
                />
            )}
        </div>
    );
}
