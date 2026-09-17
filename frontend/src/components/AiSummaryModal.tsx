import { useState, useEffect } from 'react';
import { X, Sparkles, Loader2, AlertCircle, Copy, Check } from 'lucide-react';
import { useAuthFetch } from '../context/AuthContext';
import { useTranslations, useI18n } from '../context/I18nContext';
import { copyToClipboard } from '../lib/utils';

interface AiSummaryModalProps {
    isOpen: boolean;
    onClose: () => void;
    file: {
        id: string;
        name: string;
    };
}

export function AiSummaryModal({ isOpen, onClose, file }: AiSummaryModalProps) {
    const authFetch = useAuthFetch();
    const t = useTranslations('AiModal');
    const { locale } = useI18n();
    const [loading, setLoading] = useState(true);
    const [summary, setSummary] = useState<string | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [copied, setCopied] = useState(false);

    useEffect(() => {
        if (isOpen && file) {
            generateSummary();
        }
        return () => {
            setSummary(null);
            setError(null);
            setLoading(true);
            setCopied(false);
        };
    }, [isOpen, file?.id]);

    const generateSummary = async () => {
        setLoading(true);
        setError(null);
        setSummary(null);

        try {
            const res = await authFetch('/api/ai/summarize', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    file_id: file.id,
                    max_length: 500,
                    language: locale === 'zh' ? 'zh' : 'en',
                }),
            });

            const data = await res.json();

            if (res.ok && data.success) {
                setSummary(data.content);
            } else {
                if (data.code === 'FILE_CONTENT_EMPTY' || (data.error && data.error.toLowerCase().includes('empty'))) {
                    setError(t('emptyFileError'));
                } else {
                    setError(data.error || t('failedToGenerateSummary'));
                }
            }
        } catch (err) {
            setError(t('connectError'));
        } finally {
            setLoading(false);
        }
    };

    const handleCopy = async () => {
        if (!summary) return;
        try {
            const ok = await copyToClipboard(summary);
            if (ok) {
                setCopied(true);
                setTimeout(() => setCopied(false), 2000);
            }
        } catch (err) {
            console.error('Failed to copy summary:', err);
        }
    };

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 z-[60] overflow-y-auto">
            <div className="flex min-h-screen items-center justify-center p-4">
                {/* Backdrop */}
                <div
                    className="fixed inset-0 bg-black/50 backdrop-blur-xs transition-opacity"
                    onClick={onClose}
                />

                {/* Modal */}
                <div className="relative w-full max-w-2xl bg-white dark:bg-gray-800 rounded-2xl shadow-2xl overflow-hidden">
                    {/* Header */}
                    <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                        <div className="flex items-center gap-3">
                            <div className="p-2 bg-purple-100 dark:bg-purple-900/30 rounded-xl">
                                <Sparkles className="w-5 h-5 text-purple-600 dark:text-purple-400" />
                            </div>
                            <div>
                                <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                                    {t('summaryTitle')}
                                </h2>
                                <p className="text-sm text-gray-500 dark:text-gray-400 truncate max-w-[300px]">
                                    {file.name}
                                </p>
                            </div>
                        </div>
                        <div className="flex items-center gap-2">
                            {summary && (
                                <button
                                    onClick={handleCopy}
                                    className="flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-lg text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                                    title={t('copySummary')}
                                >
                                    {copied ? (
                                        <>
                                            <Check className="w-3.5 h-3.5 text-green-500" />
                                            <span className="text-green-600 dark:text-green-400">{t('copied')}</span>
                                        </>
                                    ) : (
                                        <>
                                            <Copy className="w-3.5 h-3.5" />
                                            <span>{t('copySummary')}</span>
                                        </>
                                    )}
                                </button>
                            )}
                            <button
                                onClick={onClose}
                                className="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                            >
                                <X className="w-5 h-5 text-gray-500" />
                            </button>
                        </div>
                    </div>

                    {/* Content */}
                    <div className="p-6">
                        {loading ? (
                            <div className="flex flex-col items-center justify-center py-12">
                                <Loader2 className="w-10 h-10 text-purple-500 animate-spin mb-4" />
                                <p className="text-gray-600 dark:text-gray-400 font-medium">
                                    {t('generatingSummary')}
                                </p>
                                <p className="text-sm text-gray-400 dark:text-gray-500 mt-1">
                                    {t('generatingHint')}
                                </p>
                            </div>
                        ) : error ? (
                            <div className="flex flex-col items-center justify-center py-12">
                                <div className="p-3 bg-red-100 dark:bg-red-900/30 rounded-full mb-4">
                                    <AlertCircle className="w-8 h-8 text-red-500" />
                                </div>
                                <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-2">
                                    {t('unableToGenerateSummary')}
                                </h3>
                                <p className="text-center text-gray-600 dark:text-gray-400 max-w-md">
                                    {error}
                                </p>
                                <button
                                    onClick={generateSummary}
                                    className="mt-4 px-4 py-2 bg-purple-600 text-white rounded-lg hover:bg-purple-700 transition-colors"
                                >
                                    {t('tryAgain')}
                                </button>
                            </div>
                        ) : (
                            <div className="bg-gray-50 dark:bg-gray-900/50 rounded-xl p-4 max-h-[60vh] overflow-y-auto">
                                <p className="text-gray-800 dark:text-gray-200 whitespace-pre-wrap leading-relaxed">
                                    {summary}
                                </p>
                            </div>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
