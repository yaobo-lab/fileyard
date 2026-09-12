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
} from 'lucide-react';
import clsx from 'clsx';

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
  canDelete?: boolean;
  canShare?: boolean;
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
  canDelete = true,
  canShare = false,
}: FileContextMenuProps) {
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
  const menuHeight = 240;
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
          <Folder className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400 shrink-0" />
        ) : (
          <Eye className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400 shrink-0" />
        )}
        <span>打开</span>
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
        <span>查看信息</span>
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
        <span>下载</span>
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
        <span>重命名</span>
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
        <span>移动</span>
      </button>

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
          <span>{file.is_starred ? '取消星标' : '添加星标'}</span>
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
          <span>分享</span>
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
            <span>删除</span>
          </button>
        </>
      )}
    </div>,
    document.body
  );
}
