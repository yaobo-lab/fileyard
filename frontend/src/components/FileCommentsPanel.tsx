import { useState, useEffect, useCallback } from 'react';
import { MessageSquare, Send, Trash2, Pencil, X, CornerDownRight, Loader2, ChevronDown, ChevronUp } from 'lucide-react';
import clsx from 'clsx';
import { useAuthFetch, useAuth } from '../context/AuthContext';
import { useModalDialog } from '../context/ModalDialogContext';
import { useTranslations, useI18n } from '../context/I18nContext';
import { format } from 'date-fns';
import { zhCN, enUS } from 'date-fns/locale';

interface Comment {
    id: string;
    file_id: string;
    user_id: string;
    user_name: string;
    user_avatar?: string;
    content: string;
    parent_id?: string;
    is_edited: boolean;
    created_at: string;
    updated_at: string;
    replies: Comment[];
    can_edit: boolean;
    can_delete: boolean;
}

interface FileCommentsPanelProps {
    fileId: string;
    companyId: string;
    isExpanded?: boolean;
    variant?: 'collapsible' | 'sidebar';
    onClose?: () => void;
    onCountChange?: (count: number) => void;
}

export function FileCommentsPanel({
    fileId,
    companyId,
    isExpanded = false,
    variant = 'collapsible',
    onClose,
    onCountChange,
}: FileCommentsPanelProps) {
    const authFetch = useAuthFetch();
    const { user } = useAuth();
    const { confirm: modalConfirm } = useModalDialog();
    const t = useTranslations('Comments');
    const tCommon = useTranslations('Common');
    const { locale } = useI18n();
    const [comments, setComments] = useState<Comment[]>([]);
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [newComment, setNewComment] = useState('');
    const [replyingTo, setReplyingTo] = useState<string | null>(null);
    const [replyContent, setReplyContent] = useState('');
    const [editingId, setEditingId] = useState<string | null>(null);
    const [editContent, setEditContent] = useState('');
    const [submitting, setSubmitting] = useState(false);
    const [expanded, setExpanded] = useState(variant === 'sidebar' ? true : isExpanded);
    const [commentCount, setCommentCount] = useState(0);

    useEffect(() => {
        onCountChange?.(commentCount);
    }, [commentCount, onCountChange]);

    const fetchComments = useCallback(async () => {
        setLoading(true);
        setError(null);
        try {
            const res = await authFetch(`/api/files/${companyId}/${fileId}/comments`);
            if (res.ok) {
                const data = await res.json();
                setComments(data.comments || []);
                setCommentCount(data.total || 0);
            } else {
                setError(t('failedToLoad'));
            }
        } catch {
            setError(t('failedToLoad'));
        } finally {
            setLoading(false);
        }
    }, [authFetch, companyId, fileId, t]);

    useEffect(() => {
        if (expanded) {
            fetchComments();
        }
    }, [expanded, fetchComments]);

    // Fetch comment count on mount
    useEffect(() => {
        const fetchCount = async () => {
            try {
                const res = await authFetch(`/api/files/${companyId}/${fileId}/comments/count`);
                if (res.ok) {
                    const data = await res.json();
                    setCommentCount(data.count || 0);
                }
            } catch {
                // Ignore
            }
        };
        fetchCount();
    }, [authFetch, companyId, fileId]);

    const handleSubmitComment = async (e: React.FormEvent) => {
        e.preventDefault();
        if (!newComment.trim() || submitting) return;

        setSubmitting(true);
        try {
            const res = await authFetch(`/api/files/${companyId}/${fileId}/comments`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ content: newComment.trim() }),
            });

            if (res.ok) {
                const comment = await res.json();
                setComments((prev) => [...prev, { ...comment, replies: [] }]);
                setNewComment('');
                setCommentCount((prev) => prev + 1);
            } else {
                setError(t('failedToPost'));
            }
        } catch {
            setError(t('failedToPost'));
        } finally {
            setSubmitting(false);
        }
    };

    const handleSubmitReply = async (parentId: string) => {
        if (!replyContent.trim() || submitting) return;

        setSubmitting(true);
        try {
            const res = await authFetch(`/api/files/${companyId}/${fileId}/comments`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ content: replyContent.trim(), parent_id: parentId }),
            });

            if (res.ok) {
                const reply = await res.json();
                setComments((prev) =>
                    prev.map((c) =>
                        c.id === parentId ? { ...c, replies: [...c.replies, { ...reply, replies: [] }] } : c
                    )
                );
                setReplyContent('');
                setReplyingTo(null);
                setCommentCount((prev) => prev + 1);
            } else {
                setError(t('failedToReply'));
            }
        } catch {
            setError(t('failedToReply'));
        } finally {
            setSubmitting(false);
        }
    };

    const handleUpdateComment = async (commentId: string, parentId?: string) => {
        if (!editContent.trim() || submitting) return;

        setSubmitting(true);
        try {
            const res = await authFetch(`/api/files/${companyId}/${fileId}/comments/${commentId}`, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ content: editContent.trim() }),
            });

            if (res.ok) {
                setComments((prev) => {
                    if (parentId) {
                        return prev.map((c) =>
                            c.id === parentId
                                ? {
                                      ...c,
                                      replies: c.replies.map((r) =>
                                          r.id === commentId ? { ...r, content: editContent.trim(), is_edited: true } : r
                                      ),
                                  }
                                : c
                        );
                    }
                    return prev.map((c) => (c.id === commentId ? { ...c, content: editContent.trim(), is_edited: true } : c));
                });
                setEditingId(null);
                setEditContent('');
            } else {
                setError(t('failedToUpdate'));
            }
        } catch {
            setError(t('failedToUpdate'));
        } finally {
            setSubmitting(false);
        }
    };

    const handleDeleteComment = async (commentId: string, parentId?: string) => {
        const confirmed = await modalConfirm({
            title: tCommon('deleteConfirmTitle'),
            description: t('deleteConfirmDesc'),
            variant: 'destructive',
            confirmText: tCommon('delete'),
            cancelText: tCommon('cancel')
        });
        if (!confirmed) return;

        try {
            const res = await authFetch(`/api/files/${companyId}/${fileId}/comments/${commentId}`, {
                method: 'DELETE',
            });

            if (res.ok) {
                setComments((prev) => {
                    if (parentId) {
                        return prev.map((c) =>
                            c.id === parentId ? { ...c, replies: c.replies.filter((r) => r.id !== commentId) } : c
                        );
                    }
                    return prev.filter((c) => c.id !== commentId);
                });
                setCommentCount((prev) => Math.max(0, prev - 1));
            } else {
                setError(t('failedToDelete'));
            }
        } catch {
            setError(t('failedToDelete'));
        }
    };

    const CommentItem = ({ comment, isReply = false, parentId }: { comment: Comment; isReply?: boolean; parentId?: string }) => {
        const isEditing = editingId === comment.id;

        return (
            <div className={clsx('py-3', isReply && 'ml-6 border-l-2 border-gray-200 dark:border-gray-700 pl-4')}>
                <div className="flex items-start gap-3">
                    {/* Avatar */}
                    {comment.user_avatar ? (
                        <img src={comment.user_avatar} alt={comment.user_name} className="w-8 h-8 rounded-full object-cover" />
                    ) : (
                        <div className="w-8 h-8 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center text-xs font-medium text-primary-700 dark:text-primary-300">
                            {comment.user_name?.charAt(0)?.toUpperCase() || '?'}
                        </div>
                    )}

                    <div className="flex-1 min-w-0">
                        {/* Header */}
                        <div className="flex items-center gap-2 mb-1">
                            <span className="text-sm font-medium text-gray-900 dark:text-white">{comment.user_name}</span>
                            <span className="text-xs text-gray-500 dark:text-gray-400">
                                {format(
                                    new Date(comment.created_at),
                                    locale === 'zh' ? 'yyyy-MM-dd HH:mm' : 'MMM d, h:mm a',
                                    { locale: locale === 'zh' ? zhCN : enUS }
                                )}
                            </span>
                            {comment.is_edited && <span className="text-xs text-gray-400 dark:text-gray-500">{t('edited')}</span>}
                        </div>

                        {/* Content or Edit Form */}
                        {isEditing ? (
                            <div className="flex gap-2">
                                <input
                                    type="text"
                                    value={editContent}
                                    onChange={(e) => setEditContent(e.target.value)}
                                    className="flex-1 px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                    autoFocus
                                />
                                <button
                                    onClick={() => handleUpdateComment(comment.id, parentId)}
                                    disabled={submitting}
                                    className="px-3 py-1.5 text-xs bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50"
                                >
                                    {t('save')}
                                </button>
                                <button
                                    onClick={() => {
                                        setEditingId(null);
                                        setEditContent('');
                                    }}
                                    className="px-3 py-1.5 text-xs text-gray-600 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white"
                                >
                                    {tCommon('cancel')}
                                </button>
                            </div>
                        ) : (
                            <p className="text-sm text-gray-700 dark:text-gray-300 whitespace-pre-wrap">{comment.content}</p>
                        )}

                        {/* Actions */}
                        {!isEditing && (
                            <div className="flex items-center gap-3 mt-2">
                                {!isReply && (
                                    <button
                                        type="button"
                                        onClick={(e) => {
                                            e.stopPropagation();
                                            setReplyingTo(comment.id);
                                            setReplyContent('');
                                        }}
                                        className="text-xs text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 flex items-center gap-1 cursor-pointer transition-colors"
                                    >
                                        <CornerDownRight className="w-3 h-3" /> {t('reply')}
                                    </button>
                                )}
                                {comment.can_edit && (
                                    <button
                                        type="button"
                                        onClick={(e) => {
                                            e.stopPropagation();
                                            setEditingId(comment.id);
                                            setEditContent(comment.content);
                                        }}
                                        className="text-xs text-gray-500 hover:text-gray-700 dark:hover:text-gray-300 flex items-center gap-1 cursor-pointer transition-colors"
                                    >
                                        <Pencil className="w-3 h-3" /> {t('edit')}
                                    </button>
                                )}
                                {comment.can_delete && (
                                    <button
                                        type="button"
                                        onClick={(e) => {
                                            e.stopPropagation();
                                            handleDeleteComment(comment.id, parentId);
                                        }}
                                        className="text-xs text-red-500 hover:text-red-700 flex items-center gap-1 cursor-pointer transition-colors"
                                    >
                                        <Trash2 className="w-3 h-3" /> {t('delete')}
                                    </button>
                                )}
                            </div>
                        )}

                        {/* Reply Form */}
                        {replyingTo === comment.id && (
                            <div className="mt-3 flex gap-2">
                                <input
                                    type="text"
                                    value={replyContent}
                                    onChange={(e) => setReplyContent(e.target.value)}
                                    placeholder={t('writeReplyPlaceholder')}
                                    className="flex-1 px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                    autoFocus
                                />
                                <button
                                    onClick={() => handleSubmitReply(comment.id)}
                                    disabled={submitting || !replyContent.trim()}
                                    className="px-3 py-1.5 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50"
                                >
                                    <Send className="w-4 h-4" />
                                </button>
                                <button
                                    onClick={() => setReplyingTo(null)}
                                    className="p-1.5 text-gray-500 hover:text-gray-700 dark:hover:text-gray-300"
                                >
                                    <X className="w-4 h-4" />
                                </button>
                            </div>
                        )}
                    </div>
                </div>

                {/* Replies */}
                {comment.replies && comment.replies.length > 0 && (
                    <div className="mt-2">
                        {comment.replies.map((reply) => (
                            <CommentItem key={reply.id} comment={reply} isReply parentId={comment.id} />
                        ))}
                    </div>
                )}
            </div>
        );
    };

    if (variant === 'sidebar') {
        return (
            <div className="flex flex-col h-full bg-white dark:bg-gray-900 overflow-hidden">
                {/* Header */}
                <div className="flex items-center justify-between px-4 py-3 border-b border-gray-200/80 dark:border-gray-800 shrink-0 bg-white dark:bg-gray-900 select-none">
                    <div className="flex items-center gap-2">
                        <MessageSquare className="w-4 h-4 text-primary-600 dark:text-primary-400" />
                        <span className="text-sm font-semibold text-gray-800 dark:text-gray-200">{t('title') || '评论'}</span>
                        {commentCount > 0 && (
                            <span className="px-1.5 py-0.2 text-[11px] font-semibold bg-primary-50 dark:bg-primary-950/50 text-primary-600 dark:text-primary-400 rounded-full border border-primary-200/60 dark:border-primary-800/60">
                                {commentCount}
                            </span>
                        )}
                    </div>
                    {onClose && (
                        <button
                            type="button"
                            onClick={onClose}
                            className="p-1 rounded-md text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
                            title="收起评论"
                        >
                            <X className="w-4 h-4" />
                        </button>
                    )}
                </div>

                {/* Body - Comments Scroll Area */}
                <div className="flex-1 min-h-0 overflow-y-auto px-4 py-3 select-text">
                    {error && (
                        <div className="mb-3 p-2 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-xs rounded-lg">
                            {error}
                        </div>
                    )}

                    {loading && (
                        <div className="flex items-center justify-center py-8">
                            <Loader2 className="w-6 h-6 text-primary-600 animate-spin" />
                        </div>
                    )}

                    {!loading && comments.length > 0 && (
                        <div className="divide-y divide-gray-100 dark:divide-gray-800">
                            {comments.map((comment) => (
                                <CommentItem key={comment.id} comment={comment} />
                            ))}
                        </div>
                    )}

                    {!loading && comments.length === 0 && (
                        <div className="flex flex-col items-center justify-center py-12 text-center text-gray-400 dark:text-gray-500">
                            <div className="p-3 bg-gray-50 dark:bg-gray-800/60 rounded-full mb-2">
                                <MessageSquare className="w-6 h-6 opacity-60" />
                            </div>
                            <p className="text-xs">{t('noComments') || '暂无评论，快来发表第一条评论吧！'}</p>
                        </div>
                    )}
                </div>

                {/* Footer - New Comment Form */}
                <div className="border-t border-gray-200/80 dark:border-gray-800 p-3 bg-white dark:bg-gray-900 shrink-0 select-text">
                    <form onSubmit={handleSubmitComment} className="flex gap-2">
                        <input
                            type="text"
                            value={newComment}
                            onChange={(e) => setNewComment(e.target.value)}
                            placeholder={t('writeCommentPlaceholder') || '输入评论内容...'}
                            className="flex-1 px-3 py-2 text-xs sm:text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-800 text-gray-900 dark:text-white placeholder-gray-400 focus:outline-hidden focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500"
                        />
                        <button
                            type="submit"
                            disabled={submitting || !newComment.trim()}
                            className="px-3.5 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors shrink-0 shadow-xs"
                            title="发表"
                        >
                            {submitting ? <Loader2 className="w-4 h-4 animate-spin" /> : <Send className="w-4 h-4" />}
                        </button>
                    </form>
                </div>
            </div>
        );
    }

    return (
        <div className="border-t border-gray-200 dark:border-gray-700">
            {/* Header - Collapsible */}
            <button
                onClick={() => setExpanded(!expanded)}
                className="w-full flex items-center justify-between px-4 py-3 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
            >
                <div className="flex items-center gap-2">
                    <MessageSquare className="w-4 h-4 text-gray-500" />
                    <span className="text-sm font-medium text-gray-700 dark:text-gray-300">{t('title')}</span>
                    {commentCount > 0 && (
                        <span className="px-2 py-0.5 text-xs bg-gray-100 dark:bg-gray-700 text-gray-600 dark:text-gray-400 rounded-full">
                            {commentCount}
                        </span>
                    )}
                </div>
                {expanded ? (
                    <ChevronUp className="w-4 h-4 text-gray-400" />
                ) : (
                    <ChevronDown className="w-4 h-4 text-gray-400" />
                )}
            </button>

            {/* Expanded Content */}
            {expanded && (
                <div className="px-4 pb-4">
                    {/* Error */}
                    {error && (
                        <div className="mb-3 p-2 bg-red-50 dark:bg-red-900/20 text-red-600 dark:text-red-400 text-xs rounded-lg">
                            {error}
                        </div>
                    )}

                    {/* Loading */}
                    {loading && (
                        <div className="flex items-center justify-center py-4">
                            <Loader2 className="w-5 h-5 text-gray-400 animate-spin" />
                        </div>
                    )}

                    {/* Comments List */}
                    {!loading && comments.length > 0 && (
                        <div className="divide-y divide-gray-100 dark:divide-gray-700 mb-4 max-h-64 overflow-y-auto">
                            {comments.map((comment) => (
                                <CommentItem key={comment.id} comment={comment} />
                            ))}
                        </div>
                    )}

                    {/* Empty State */}
                    {!loading && comments.length === 0 && (
                        <div className="text-center py-4 text-sm text-gray-500 dark:text-gray-400">
                            {t('noComments')}
                        </div>
                    )}

                    {/* New Comment Form */}
                    <form onSubmit={handleSubmitComment} className="flex gap-2">
                        <input
                            type="text"
                            value={newComment}
                            onChange={(e) => setNewComment(e.target.value)}
                            placeholder={t('writeCommentPlaceholder')}
                            className="flex-1 px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400"
                        />
                        <button
                            type="submit"
                            disabled={submitting || !newComment.trim()}
                            className="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                        >
                            {submitting ? <Loader2 className="w-4 h-4 animate-spin" /> : <Send className="w-4 h-4" />}
                        </button>
                    </form>
                </div>
            )}
        </div>
    );
}

