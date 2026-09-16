import React, { useEffect, useRef, useState, useLayoutEffect } from 'react';
import { createPortal } from 'react-dom';
import {
  Folder,
  Eye,
  Info,
  Download,
  Edit2,
  Move,
  Trash2,
  Star,
  Share2,
  FileText,
  Sparkles,
  MessageSquare,
} from 'lucide-react';
import { FileSystemFolderGlyph } from './FileGlyphs';
import clsx from 'clsx';
import { useTranslations } from '../context/I18nContext';

export interface ContextMenuTarget {
  x: number;
  y: number;
  file: any;
}

interface FileContextMenuProps {
  target: ContextMenuTarget | null;
  onClose: () => void;
  onOpen: (file: any) => void;
  onProperties: (file: any) => void;
  onDownload: (file: any) => void;
  onRename: (file: any) => void;
  onMove: (file: any) => void;
  onDelete: (file: any) => void;
  onStar?: (file: any) => void;
  onShare?: (file: any) => void;
  onConvertToMarkdown?: (file: any) => void;
  onAiSummarize?: (file: any) => void;
  onAiQuestion?: (file: any) => void;
  aiEnabled?: boolean;
  canUseAi?: boolean;
  canDelete?: boolean;
  canShare?: boolean;
}

const SUPPORTED_MARKDOWN_EXTENSIONS = new Set([
  // Word documents
  'docx', 'doc', 'docm',
  // PowerPoint presentations
  'pptx', 'ppt', 'pptm', 'pps', 'ppsx', 'ppsm',
  // Excel spreadsheets
  'xlsx', 'xls', 'xlsm', 'xlsb',
  // OpenDocument formats
  'odt', 'ods', 'odp',
  // PDF
  'pdf',
  // Rich Text, E-books, CSV, Plain text
  'rtf', 'epub', 'csv', 'tsv', 'txt'
]);

const SUPPORTED_AI_EXTENSIONS = new Set([
  'txt', 'md', 'markdown', 'json', 'xml', 'csv', 'tsv',
  'html', 'htm', 'js', 'ts', 'jsx', 'tsx', 'py', 'rs', 'go',
  'java', 'c', 'cpp', 'h', 'hpp', 'css', 'scss', 'yaml', 'yml',
  'pdf', 'docx', 'xlsx', 'pptx', 'rtf', 'log', 'sql', 'sh'
]);

export function canUseAiOnFile(file: any): boolean {
  if (!file) return false;
  if (file.type === 'folder' || file.kind === 'folder' || file.type === 'group') return false;
  const fileName = file.name || '';
  const ext = fileName.toLowerCase().split('.').pop() || '';
  if (SUPPORTED_AI_EXTENSIONS.has(ext)) return true;
  if (file.content_type) {
    if (file.content_type.startsWith('text/')) return true;
    if (['application/json', 'application/xml', 'application/pdf'].includes(file.content_type)) return true;
  }
  return false;
}

export function canConvertToMarkdown(file: any): boolean {
  if (!file) return false;
  if (file.type === 'folder' || file.kind === 'folder' || file.type === 'group') return false;
  const fileName = file.name || '';
  const ext = fileName.toLowerCase().split('.').pop() || '';
  return SUPPORTED_MARKDOWN_EXTENSIONS.has(ext);
}

export function FileContextMenu({
  target,
  onClose,
  onOpen,
  onProperties,
  onDownload,
  onRename,
  onMove,
  onDelete,
  onStar,
  onShare,
  onConvertToMarkdown,
  onAiSummarize,
  onAiQuestion,
  aiEnabled = false,
  canUseAi = false,
  canDelete = true,
  canShare = false,
}: FileContextMenuProps) {
  const t = useTranslations('ContextMenu');
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!target) return;

    const handleOutside = (e: MouseEvent | TouchEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };

    const handleScroll = () => {
      onClose();
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    // Use capturing listeners to reliably catch clicks and dismiss
    document.addEventListener('mousedown', handleOutside, true);
    document.addEventListener('scroll', handleScroll, true);
    document.addEventListener('keydown', handleKeyDown);

    return () => {
      document.removeEventListener('mousedown', handleOutside, true);
      document.removeEventListener('scroll', handleScroll, true);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [target, onClose]);

  if (!target) return null;

  const { file } = target;
  const isFolder = file.type === 'folder' || file.kind === 'folder';

  // Synchronously calculate boundary-safe coordinates on render without jump or float
  const menuWidth = 160;
  const menuHeight = 280;
  const padding = 8;
  const left = target.x + menuWidth > window.innerWidth - padding
    ? Math.max(padding, window.innerWidth - menuWidth - padding)
    : target.x;
  const top = target.y + menuHeight > window.innerHeight - padding
    ? Math.max(padding, window.innerHeight - menuHeight - padding)
    : target.y;

  return createPortal(
    <div
      ref={menuRef}
      role="menu"
      aria-orientation="vertical"
      className="fixed z-[99999] min-w-[156px] rounded-xl border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 p-1 shadow-lg text-xs select-none"
      style={{ left, top }}
      onClick={(e) => e.stopPropagation()}
      onContextMenu={(e) => e.preventDefault()}
    >
      {/* 打开 */}
      <button
        type="button"
        role="menuitem"
        className="flex w-full items-center gap-2.5 rounded-lg px-3 py-1.5 text-xs text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors text-left font-normal"
        onClick={() => {
          onClose();
          onOpen(file);
        }}
      >
        {isFolder ? (
          <FileSystemFolderGlyph size="xs" className="h-3.5 w-auto shrink-0" />
        ) : (
          <Eye className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400 shrink-0" />
        )}
        <span>{t('open')}</span>
      </button>

      <div className="my-1 border-t border-gray-100 dark:border-gray-700/60" />

      {/* 查看信息 */}
      <button
        type="button"
        role="menuitem"
        className="flex w-full items-center gap-2.5 rounded-lg px-3 py-1.5 text-xs text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors text-left font-normal"
        onClick={() => {
          onClose();
          onProperties(file);
        }}
      >
        <Info className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400 shrink-0" />
        <span>{t('viewInfo')}</span>
      </button>

      {/* 下载 */}
      <button
        type="button"
        role="menuitem"
        className="flex w-full items-center gap-2.5 rounded-lg px-3 py-1.5 text-xs text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors text-left font-normal"
        onClick={() => {
          onClose();
          onDownload(file);
        }}
      >
        <Download className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400 shrink-0" />
        <span>{t('download')}</span>
      </button>

      {/* 重命名 */}
      <button
        type="button"
        role="menuitem"
        className="flex w-full items-center gap-2.5 rounded-lg px-3 py-1.5 text-xs text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors text-left font-normal"
        onClick={() => {
          onClose();
          onRename(file);
        }}
      >
        <Edit2 className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400 shrink-0" />
        <span>{t('rename')}</span>
      </button>

      {/* 移动 */}
      <button
        type="button"
        role="menuitem"
        className="flex w-full items-center gap-2.5 rounded-lg px-3 py-1.5 text-xs text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors text-left font-normal"
        onClick={() => {
          onClose();
          onMove(file);
        }}
      >
        <Move className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400 shrink-0" />
        <span>{t('move')}</span>
      </button>

      {/* 转 Markdown */}
      {canConvertToMarkdown(file) && onConvertToMarkdown && (
        <button
          type="button"
          role="menuitem"
          className="flex w-full items-center gap-2.5 rounded-lg px-3 py-1.5 text-xs text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors text-left font-normal"
          onClick={() => {
            onClose();
            onConvertToMarkdown(file);
          }}
        >
          <FileText className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400 shrink-0" />
          <span>{t('convertToMarkdown')}</span>
        </button>
      )}

      {/* AI 智能摘要 */}
      {Boolean(aiEnabled && canUseAi) && canUseAiOnFile(file) && onAiSummarize && (
        <button
          type="button"
          role="menuitem"
          className="flex w-full items-center gap-2.5 rounded-lg px-3 py-1.5 text-xs text-purple-700 dark:text-purple-300 hover:bg-purple-50 dark:hover:bg-purple-900/20 transition-colors text-left font-normal"
          onClick={() => {
            onClose();
            onAiSummarize(file);
          }}
        >
          <Sparkles className="w-3.5 h-3.5 text-purple-500 shrink-0" />
          <span>{t('aiSummarize')}</span>
        </button>
      )}

      {/* AI 智能问答 */}
      {Boolean(aiEnabled && canUseAi) && canUseAiOnFile(file) && onAiQuestion && (
        <button
          type="button"
          role="menuitem"
          className="flex w-full items-center gap-2.5 rounded-lg px-3 py-1.5 text-xs text-purple-700 dark:text-purple-300 hover:bg-purple-50 dark:hover:bg-purple-900/20 transition-colors text-left font-normal"
          onClick={() => {
            onClose();
            onAiQuestion(file);
          }}
        >
          <MessageSquare className="w-3.5 h-3.5 text-purple-500 shrink-0" />
          <span>{t('aiQuestion')}</span>
        </button>
      )}

      {/* 收藏 / 星标 */}
      {onStar && (
        <button
          type="button"
          role="menuitem"
          className="flex w-full items-center gap-2.5 rounded-lg px-3 py-1.5 text-xs text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors text-left font-normal"
          onClick={() => {
            onClose();
            onStar(file);
          }}
        >
          <Star
            className={clsx(
              "w-3.5 h-3.5 shrink-0",
              file.is_starred ? "text-yellow-400 fill-yellow-400" : "text-gray-500 dark:text-gray-400"
            )}
          />
          <span>{file.is_starred ? t('removeStar') : t('addStar')}</span>
        </button>
      )}

      {/* 分享 (可选) */}
      {canShare && onShare && (
        <button
          type="button"
          role="menuitem"
          className="flex w-full items-center gap-2.5 rounded-lg px-3 py-1.5 text-xs text-gray-700 dark:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors text-left font-normal"
          onClick={() => {
            onClose();
            onShare(file);
          }}
        >
          <Share2 className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400 shrink-0" />
          <span>{t('share')}</span>
        </button>
      )}

      {/* 删除 */}
      {canDelete && (
        <>
          <div className="my-1 border-t border-gray-100 dark:border-gray-700/60" />
          <button
            type="button"
            role="menuitem"
            className="flex w-full items-center gap-2.5 rounded-lg px-3 py-1.5 text-xs font-normal text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-950/40 transition-colors text-left"
            onClick={() => {
              onClose();
              onDelete(file);
            }}
          >
            <Trash2 className="w-3.5 h-3.5 text-red-500 shrink-0" />
            <span>{t('delete')}</span>
          </button>
        </>
      )}
    </div>,
    document.body
  );
}
