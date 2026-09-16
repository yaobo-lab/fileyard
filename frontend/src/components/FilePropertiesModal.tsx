import { X, Folder, FileText, Image, Film, Music, Lock, Eye, EyeOff, Calendar, User, HardDrive, Building } from 'lucide-react';
import { format } from 'date-fns';
import { FileCommentsPanel } from './FileCommentsPanel';
import { FileGlyphVisual } from './FileGlyphs';
import { useTranslations, useI18n } from '../context/I18nContext';
import { zhCN, enUS } from 'date-fns/locale';

interface FileItem {
    id: string;
    name: string;
    type: 'folder' | 'image' | 'document' | 'video' | 'audio' | 'group';
    size?: string;
    size_bytes?: number;
    modified: string;
    created_at?: string;
    owner: string;
    owner_id?: string;
    owner_avatar?: string;
    is_locked?: boolean;
    locked_by?: string;
    locked_at?: string;
    lock_requires_role?: string;
    visibility?: 'department' | 'private';
    department_id?: string;
    content_type?: string;
    storage_path?: string;
    color?: string;
    file_count?: number;
}

interface FilePropertiesModalProps {
    isOpen: boolean;
    onClose: () => void;
    file: FileItem | null;
    departmentName?: string;
    companyId?: string;
}

const getFileIcon = (file: FileItem, companyId?: string) => {
    return <FileGlyphVisual file={file} companyId={companyId} size="md" />;
};

export function FilePropertiesModal({ isOpen, onClose, file, departmentName, companyId }: FilePropertiesModalProps) {
    const t = useTranslations('Properties');
    const tCommon = useTranslations('Common');
    const { locale } = useI18n();
    if (!isOpen || !file) return null;

    const formatDate = (dateStr?: string) => {
        if (!dateStr) return t('unknown');
        try {
            return format(new Date(dateStr), 'PPpp', { locale: locale === 'zh' ? zhCN : enUS });
        } catch {
            return dateStr;
        }
    };

    const PropertyRow = ({ icon: Icon, label, value, className = '' }: { icon: any; label: string; value: React.ReactNode; className?: string }) => (
        <div className="flex items-start py-3 border-b border-gray-100 dark:border-gray-700 last:border-0">
            <Icon className="w-4 h-4 text-gray-400 mt-0.5 mr-3 flex-shrink-0" />
            <div className="flex-1 min-w-0">
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-0.5">{label}</p>
                <p className={`text-sm text-gray-900 dark:text-white ${className}`}>{value}</p>
            </div>
        </div>
    );

    return (
        <div className="fixed inset-0 z-[60] overflow-y-auto">
            <div className="flex min-h-full items-center justify-center p-4">
                {/* Backdrop */}
                <div 
                    className="fixed inset-0 bg-black/50 backdrop-blur-xs transition-opacity" 
                />

                {/* Modal */}
                <div className="relative bg-white dark:bg-gray-800 rounded-xl shadow-xl w-full max-w-md transform transition-all">
                    {/* Header */}
                    <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                        <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                            {t('title')}
                        </h2>
                        <button
                            onClick={onClose}
                            className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                        >
                            <X className="w-5 h-5" />
                        </button>
                    </div>

                    {/* Content */}
                    <div className="px-6 py-4">
                        {/* File Icon and Name */}
                        <div className="flex items-center space-x-4 pb-4 border-b border-gray-200 dark:border-gray-700 mb-4">
                            <div className="flex-shrink-0 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-xl">
                                {getFileIcon(file, companyId)}
                            </div>
                            <div className="flex-1 min-w-0">
                                <h3 className="text-base font-medium text-gray-900 dark:text-white truncate" title={file.name}>
                                    {file.name}
                                </h3>
                                <p className="text-sm text-gray-500 dark:text-gray-400 capitalize">
                                    {file.type === 'folder' ? t('folder') : file.content_type || t('fileType', { type: file.type })}
                                </p>
                            </div>
                        </div>

                        {/* Properties List */}
                        <div className="space-y-0">
                            {/* Owner, Size, Visibility in One Row */}
                            <div className="grid grid-cols-3 gap-3 py-3 border-b border-gray-100 dark:border-gray-700">
                                {/* Owner */}
                                <div className="min-w-0">
                                    <div className="flex items-center text-xs text-gray-500 dark:text-gray-400 mb-1">
                                        <User className="w-3.5 h-3.5 mr-1 text-gray-400 shrink-0" />
                                        <span>{t('owner')}</span>
                                    </div>
                                    <div className="flex items-center gap-1.5 min-w-0">
                                        {file.owner_avatar ? (
                                            <img 
                                                src={file.owner_avatar} 
                                                alt={file.owner}
                                                className="w-5 h-5 rounded-full object-cover shrink-0"
                                            />
                                        ) : (
                                            <div className="w-5 h-5 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center text-[10px] font-medium text-primary-700 dark:text-primary-300 shrink-0">
                                                {file.owner?.charAt(0)?.toUpperCase() || '?'}
                                            </div>
                                        )}
                                        <span className="text-sm font-medium text-gray-900 dark:text-white truncate" title={file.owner}>
                                            {file.owner || t('unknown')}
                                        </span>
                                    </div>
                                </div>

                                {/* Size */}
                                <div className="min-w-0">
                                    <div className="flex items-center text-xs text-gray-500 dark:text-gray-400 mb-1">
                                        <HardDrive className="w-3.5 h-3.5 mr-1 text-gray-400 shrink-0" />
                                        <span>{t('size')}</span>
                                    </div>
                                    <p className="text-sm font-medium text-gray-900 dark:text-white truncate" title={file.size}>
                                        {file.size || (file.type === 'folder' ? t('calculating') : t('unknown'))}
                                    </p>
                                </div>

                                {/* Visibility */}
                                <div className="min-w-0">
                                    <div className="flex items-center text-xs text-gray-500 dark:text-gray-400 mb-1">
                                        {file.visibility === 'private' ? (
                                            <EyeOff className="w-3.5 h-3.5 mr-1 text-purple-500 shrink-0" />
                                        ) : (
                                            <Eye className="w-3.5 h-3.5 mr-1 text-gray-400 shrink-0" />
                                        )}
                                        <span>{t('visibility')}</span>
                                    </div>
                                    <div>
                                        {file.visibility === 'private' ? (
                                            <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-purple-100 dark:bg-purple-900/30 text-purple-800 dark:text-purple-300">
                                                {t('private')}
                                            </span>
                                        ) : (
                                            <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-300">
                                                {t('department')}
                                            </span>
                                        )}
                                    </div>
                                </div>
                            </div>

                            {/* Department (if department visibility) */}
                            {file.visibility === 'department' && departmentName && (
                                <PropertyRow 
                                    icon={Building} 
                                    label={t('department')} 
                                    value={departmentName} 
                                />
                            )}

                            {/* Lock Status */}
                            {file.is_locked && (
                                <div className="flex items-start py-3 border-b border-gray-100 dark:border-gray-700">
                                    <Lock className="w-4 h-4 text-orange-500 mt-0.5 mr-3 flex-shrink-0" />
                                    <div className="flex-1 min-w-0">
                                        <p className="text-xs text-gray-500 dark:text-gray-400 mb-0.5">{t('lockStatus')}</p>
                                        <p className="text-sm text-gray-900 dark:text-white">
                                            <span className="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-orange-100 dark:bg-orange-900/30 text-orange-800 dark:text-orange-300">
                                                {t('locked')}
                                            </span>
                                            {file.lock_requires_role && (
                                                <span className="ml-2 text-xs text-gray-500">
                                                    ({t('requiredRole', { role: file.lock_requires_role })})
                                                </span>
                                            )}
                                        </p>
                                        {file.locked_at && (
                                            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                                                {t('lockedOn', { time: formatDate(file.locked_at) })}
                                            </p>
                                        )}
                                    </div>
                                </div>
                            )}

                            {/* Created Date */}
                            <PropertyRow 
                                icon={Calendar} 
                                label={t('created')} 
                                value={formatDate(file.created_at)} 
                            />

                            {/* Modified Date */}
                            <PropertyRow 
                                icon={Calendar} 
                                label={t('modified')} 
                                value={formatDate(file.modified)} 
                            />
                        </div>
                    </div>

                    {/* Comments Section - only for files, not folders */}
                    {file.type !== 'folder' && companyId && (
                        <FileCommentsPanel fileId={file.id} companyId={companyId} />
                    )}

                    {/* Footer */}
                    <div className="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex justify-end">
                        <button
                            onClick={onClose}
                            className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-200 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-600 transition-colors"
                        >
                            {t('close')}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}
