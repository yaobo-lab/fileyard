import React, { useState, useEffect, useRef, useCallback } from 'react';
import {
  X,
  Download,
  Loader2,
  ExternalLink,
  Search,
  Maximize2,
  Minimize2,
  FileText,
  FileSpreadsheet,
  AlertCircle,
  Star,
  ChevronLeft,
  ChevronRight,
  ZoomIn,
  ZoomOut,
  RotateCcw,
  RotateCw,
  MessageSquare,
} from 'lucide-react';
import clsx from 'clsx';
import { CodeViewer, CodeViewerHandle } from './viewers/CodeViewer';
import { DocxViewer } from './viewers/DocxViewer';
import { XlsxViewer } from './viewers/XlsxViewer';
import { PptxViewer } from './viewers/PptxViewer';
import { DrawioViewer } from './viewers/DrawioViewer';
import { MarkdownViewer } from './viewers/MarkdownViewer';
import { FileGlyphVisual } from './FileGlyphs';
import { FileCommentsPanel } from './FileCommentsPanel';

interface FilePreviewModalProps {
  isOpen: boolean;
  onClose: () => void;
  file: {
    id?: string;
    companyId?: string;
    parentPath?: string;
    name: string;
    url: string;
    type?: string;
    size?: number | string;
    initialMode?: 'rendered' | 'source' | 'split';
  } | null;
  onSaved?: (newMetadata?: any) => void;
  onPrev?: () => void;
  onNext?: () => void;
  hasPrev?: boolean;
  hasNext?: boolean;
  itemIndex?: number;
  itemCount?: number;
  isStarred?: boolean;
  onToggleStar?: () => void;
}

export type SupportedKind =
  | 'text'
  | 'markdown'
  | 'pdf'
  | 'docx'
  | 'xlsx'
  | 'pptx'
  | 'drawio'
  | 'image'
  | 'video'
  | 'audio'
  | 'other';

export function detectFileKind(fileName: string, type?: string): SupportedKind {
  const lower = fileName.toLowerCase();

  if (lower.endsWith('.md') || lower.endsWith('.markdown') || lower.endsWith('.mdx')) {
    return 'markdown';
  }

  if (lower.endsWith('.docx') || lower.endsWith('.doc')) return 'docx';
  if (lower.endsWith('.pptx') || lower.endsWith('.ppt')) return 'pptx';
  if (lower.endsWith('.xlsx') || lower.endsWith('.xls') || lower.endsWith('.xlsm') || lower.endsWith('.csv')) {
    // Note: CSV can be viewed as spreadsheet or code, default to xlsx if requested, but CSV works in CodeViewer too
    if (lower.endsWith('.csv')) return 'text';
    return 'xlsx';
  }

  if (lower.endsWith('.pdf')) return 'pdf';
  if (lower.endsWith('.drawio') || lower.endsWith('.dio')) return 'drawio';

  if (type === 'image' || /\.(avif|gif|jpe?g|png|svg|webp|bmp|ico)$/i.test(lower)) return 'image';
  if (type === 'video' || /\.(mp4|webm|ogv|mov|m4v|mkv|avi)$/i.test(lower)) return 'video';
  if (type === 'audio' || /\.(aac|aif|aiff|flac|m4a|mp3|oga|ogg|opus|wav|weba)$/i.test(lower)) return 'audio';

  // Comprehensive code and text formats
  const TEXT_EXTS = new Set([
    'txt', 'text', 'json', 'jsonc', 'json5', 'csv', 'tsv',
    'xml', 'html', 'htm', 'xhtml', 'css', 'scss', 'sass', 'less', 'js', 'jsx', 'mjs',
    'cjs', 'ts', 'tsx', 'mts', 'cts', 'py', 'rs', 'go', 'java', 'kt', 'kts', 'c',
    'h', 'cc', 'cpp', 'hpp', 'cxx', 'cs', 'php', 'rb', 'sh', 'bash', 'zsh', 'fish',
    'ps1', 'bat', 'cmd', 'yaml', 'yml', 'toml', 'ini', 'conf', 'cfg', 'properties',
    'env', 'log', 'sql', 'vue', 'svelte', 'astro', 'dockerfile', 'makefile', 'r', 'dart',
    'lua', 'swift', 'scala', 'pl', 'pm', 'graphql'
  ]);
  const ext = lower.split('.').pop() || '';
  if (TEXT_EXTS.has(ext)) return 'text';

  return 'other';
}

const MODAL_SIZES: Record<SupportedKind, string> = {
  markdown: 'h-[88vh] w-[min(96vw,76rem)]',
  text: 'h-[85vh] w-[min(96vw,80rem)]',
  pdf: 'h-[88vh] w-[min(96vw,68rem)]',
  docx: 'h-[88vh] w-[min(96vw,68rem)]',
  pptx: 'h-[88vh] w-[min(96vw,84rem)]',
  xlsx: 'h-[85vh] w-[min(96vw,100rem)]',
  drawio: 'h-[88vh] w-[min(96vw,84rem)]',
  image: 'h-[88vh] w-[min(96vw,72rem)]',
  video: 'w-[min(96vw,72rem)] max-w-none',
  audio: 'w-[min(96vw,36rem)] max-w-none',
  other: 'max-w-md',
};

export function FilePreviewModal({
  isOpen,
  onClose,
  file,
  onSaved,
  onPrev,
  onNext,
  hasPrev = false,
  hasNext = false,
  itemIndex,
  itemCount,
  isStarred,
  onToggleStar,
}: FilePreviewModalProps) {
  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [internalStarred, setInternalStarred] = useState(false);
  const starredActive = typeof isStarred === 'boolean' ? isStarred : internalStarred;
  const handleToggleStar = () => {
    if (onToggleStar) {
      onToggleStar();
    } else {
      setInternalStarred((prev) => !prev);
    }
  };
  const [isMarkdownDirty, setIsMarkdownDirty] = useState(false);
  const [scale, setScale] = useState(1);
  const [rotation, setRotation] = useState(0);
  const [isCommentsOpen, setIsCommentsOpen] = useState(false);
  const [commentCount, setCommentCount] = useState<number>(0);
  const codeViewerRef = useRef<CodeViewerHandle>(null);

  // Fetch comment count when file changes
  useEffect(() => {
    if (!file?.id || !file?.companyId) {
      setCommentCount(0);
      return;
    }
    let isMounted = true;
    const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
    fetch(`/api/files/${file.companyId}/${file.id}/comments/count`, {
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    })
      .then((res) => (res.ok ? res.json() : null))
      .then((data) => {
        if (isMounted && data && typeof data.count === 'number') {
          setCommentCount(data.count);
        }
      })
      .catch(() => {});
    return () => {
      isMounted = false;
    };
  }, [file?.id, file?.companyId]);

  // Reset scale and rotation on image change
  useEffect(() => {
    setScale(1);
    setRotation(0);
  }, [file?.url, file?.name]);

  const handleZoomIn = () => setScale((prev) => Math.min(prev + 0.25, 4));
  const handleZoomOut = () => setScale((prev) => Math.max(prev - 0.25, 0.25));
  const handleResetZoom = () => {
    setScale(1);
    setRotation(0);
  };
  const handleRotateCw = () => setRotation((prev) => (prev + 90) % 360);
  const handleRotateCcw = () => setRotation((prev) => (prev - 90 + 360) % 360);

  const handleRequestClose = useCallback(() => {
    if (isMarkdownDirty) {
      if (!window.confirm('当前文档有尚未保存的内容，确定要退出吗？未保存的修改将会丢失。')) {
        return;
      }
    }
    onClose();
  }, [isMarkdownDirty, onClose]);

  // Drag position state
  const [position, setPosition] = useState({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState(false);
  const dragStartRef = useRef<{ startX: number; startY: number; posX: number; posY: number } | null>(null);

  // Reset position when closed, file changed, or fullscreen
  useEffect(() => {
    if (!isOpen || isFullscreen) {
      setPosition({ x: 0, y: 0 });
    }
  }, [isOpen, file?.name, isFullscreen]);

  const handleHeaderPointerDown = (e: React.PointerEvent) => {
    if (isFullscreen) return;
    if ((e.target as HTMLElement).closest('button, a, input')) return;

    e.preventDefault();
    setIsDragging(true);
    dragStartRef.current = {
      startX: e.clientX,
      startY: e.clientY,
      posX: position.x,
      posY: position.y,
    };

    const handlePointerMove = (moveEvent: PointerEvent) => {
      if (!dragStartRef.current) return;
      const dx = moveEvent.clientX - dragStartRef.current.startX;
      const dy = moveEvent.clientY - dragStartRef.current.startY;
      setPosition({
        x: dragStartRef.current.posX + dx,
        y: dragStartRef.current.posY + dy,
      });
    };

    const handlePointerUp = () => {
      setIsDragging(false);
      dragStartRef.current = null;
      window.removeEventListener('pointermove', handlePointerMove);
      window.removeEventListener('pointerup', handlePointerUp);
    };

    window.addEventListener('pointermove', handlePointerMove);
    window.addEventListener('pointerup', handlePointerUp);
  };

  // Detect dark mode from document
  const [isDark, setIsDark] = useState(() => document.documentElement.classList.contains('dark'));
  useEffect(() => {
    const observer = new MutationObserver(() => {
      setIsDark(document.documentElement.classList.contains('dark'));
    });
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!isOpen || !file) {
      if (blobUrl) {
        URL.revokeObjectURL(blobUrl);
        setBlobUrl(null);
      }
      setError(null);
      setLoading(false);
      setIsFullscreen(false);
      return;
    }

    let isMounted = true;
    setLoading(true);
    setError(null);

    const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
    const previewUrl = file.url.includes('?') ? `${file.url}&preview=true` : `${file.url}?preview=true`;

    fetch(previewUrl, {
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    })
      .then((res) => {
        if (!res.ok) throw new Error(`加载文件失败 (${res.status})`);
        return res.blob();
      })
      .then((blob) => {
        if (!isMounted) return;
        const namedFile = new File([blob], file.name, { type: blob.type });
        const objectUrl = URL.createObjectURL(namedFile);
        setBlobUrl(objectUrl);
      })
      .catch((err) => {
        if (!isMounted) return;
        setError(err.message || '文件加载失败，请尝试下载后查看');
      })
      .finally(() => {
        if (isMounted) setLoading(false);
      });

    return () => {
      isMounted = false;
      if (blobUrl) {
        URL.revokeObjectURL(blobUrl);
      }
    };
  }, [isOpen, file?.url, file?.name]);

  const kind = file ? detectFileKind(file.name, file.type) : 'other';

  // Keyboard navigation for image and general shortcuts
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (['INPUT', 'TEXTAREA'].includes((e.target as HTMLElement)?.tagName)) {
        return;
      }
      if (e.key === 'Escape') {
        handleRequestClose();
        return;
      }
      if (kind === 'image') {
        if (e.key === 'ArrowLeft') {
          if (hasPrev && onPrev) {
            e.preventDefault();
            onPrev();
          }
        } else if (e.key === 'ArrowRight') {
          if (hasNext && onNext) {
            e.preventDefault();
            onNext();
          }
        } else if (e.key === '+' || e.key === '=') {
          e.preventDefault();
          handleZoomIn();
        } else if (e.key === '-' || e.key === '_') {
          e.preventDefault();
          handleZoomOut();
        } else if (e.key === '0') {
          e.preventDefault();
          handleResetZoom();
        } else if (e.key === 'r' || e.key === 'R') {
          e.preventDefault();
          handleRotateCw();
        }
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, kind, hasPrev, hasNext, onPrev, onNext, handleRequestClose]);

  if (!isOpen || !file) return null;

  const isMedia = kind === 'image' || kind === 'video' || kind === 'audio';

  const handleDownload = () => {
    if (blobUrl) {
      const a = document.createElement('a');
      a.href = blobUrl;
      a.download = file.name;
      a.click();
    }
  };

  const handleOpenInNewTab = () => {
    if (blobUrl) {
      window.open(blobUrl, '_blank');
    }
  };

  return (
    <div
      className={clsx(
        "fixed inset-0 z-[100] flex items-center justify-center bg-black/50 backdrop-blur-xs transition-opacity animate-in fade-in duration-100",
        isFullscreen ? "p-0" : "p-3 sm:p-5"
      )}
    >
      <div
        style={!isFullscreen ? { transform: `translate3d(${position.x}px, ${position.y}px, 0)` } : undefined}
        className={clsx(
          'relative flex flex-col bg-white dark:bg-gray-900 shadow-2xl border border-gray-200/80 dark:border-gray-800 overflow-hidden',
          isFullscreen
            ? 'w-screen h-screen rounded-none border-0'
            : isCommentsOpen
              ? 'h-[88vh] w-[min(96vw,84rem)] rounded-2xl'
              : `${MODAL_SIZES[kind]} rounded-2xl`,
          isDragging ? 'transition-none select-none' : 'transition-all duration-150'
        )}
      >
        {/* Top Header Bar (Matching screenshot 1:1, draggable) */}
        <div
          onPointerDown={handleHeaderPointerDown}
          onDoubleClick={() => setIsFullscreen(!isFullscreen)}
          className={clsx(
            "flex h-11 shrink-0 items-center justify-between border-b border-gray-200/80 dark:border-gray-800 px-4 bg-white dark:bg-gray-900 select-none",
            !isFullscreen ? (isDragging ? "cursor-grabbing" : "cursor-grab") : "cursor-default"
          )}
          title={!isFullscreen ? "按住可拖动窗口，双击全屏" : "双击还原窗口"}
        >
          <div className="flex items-center min-w-0 pr-3">
            <h3 className="text-sm font-medium text-gray-800 dark:text-gray-200 truncate" title={file.name}>
              {file.name}
            </h3>
            {kind === 'image' && typeof itemCount === 'number' && itemCount > 1 && (
              <span className="ml-2 px-1.5 py-0.5 rounded text-[11px] font-medium bg-gray-100 dark:bg-gray-800 text-gray-500 dark:text-gray-400 shrink-0">
                {(itemIndex ?? 0) + 1} / {itemCount}
              </span>
            )}
          </div>

          <div className="flex items-center gap-2 shrink-0">
            {/* Search (for code/text) */}
            {kind === 'text' && (
              <button
                type="button"
                onClick={() => codeViewerRef.current?.toggleSearch()}
                className="p-1 rounded text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
                title="搜索 (Ctrl+F)"
              >
                <Search className="w-4 h-4" />
              </button>
            )}

            {/* File Comments Toggle Button */}
            {file.id && file.companyId && (
              <button
                type="button"
                onClick={() => setIsCommentsOpen(!isCommentsOpen)}
                className={clsx(
                  "relative p-1.5 rounded-md border transition-colors shadow-2xs",
                  isCommentsOpen
                    ? "bg-primary-50 dark:bg-primary-950/40 text-primary-600 dark:text-primary-400 border-primary-300 dark:border-primary-700"
                    : "border-gray-250 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white"
                )}
                title={isCommentsOpen ? "收起评论" : "文件评论"}
              >
                <MessageSquare className="w-3.5 h-3.5" />
                {commentCount > 0 && (
                  <span className="absolute -top-1.5 -right-1.5 px-1 min-w-[15px] h-3.5 flex items-center justify-center text-[9px] font-bold bg-primary-600 text-white rounded-full border border-white dark:border-gray-900">
                    {commentCount > 99 ? '99+' : commentCount}
                  </span>
                )}
              </button>
            )}

            {/* Favorite Star Button (matching screenshot) */}
            <button
              type="button"
              onClick={handleToggleStar}
              className="p-1.5 rounded-md border border-gray-250 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-500 transition-colors shadow-2xs"
              title={starredActive ? '取消收藏' : '添加收藏'}
            >
              <Star className={clsx("w-3.5 h-3.5", starredActive ? "fill-amber-400 text-amber-500" : "text-gray-500 dark:text-gray-400")} />
            </button>

            {/* Fullscreen Maximize / Minimize Button */}
            <button
              type="button"
              onClick={() => setIsFullscreen(!isFullscreen)}
              className="p-1.5 rounded-md border border-gray-250 dark:border-gray-700 hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white transition-colors shadow-2xs"
              title={isFullscreen ? '还原窗口' : '全屏放大'}
            >
              {isFullscreen ? (
                <Minimize2 className="w-3.5 h-3.5" />
              ) : (
                <Maximize2 className="w-3.5 h-3.5" />
              )}
            </button>

            {/* Close Button (matching screenshot) */}
            <button
              type="button"
              onClick={handleRequestClose}
              className="p-1 rounded text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
              title="关闭 (Esc)"
            >
              <X className="w-5 h-5" />
            </button>
          </div>
        </div>

        {/* Content Viewer Body Container with optional Comments Sidebar */}
        <div className="relative flex-1 min-h-0 min-w-0 flex overflow-hidden">
          {/* Main Viewer Canvas */}
          <div className="relative flex-1 min-h-0 min-w-0 overflow-hidden bg-gray-100/50 dark:bg-gray-900/50">
          {loading ? (
            <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center text-gray-500">
              <Loader2 className="w-8 h-8 animate-spin text-primary-600" />
              <p className="text-sm font-medium">正在准备预览文档...</p>
            </div>
          ) : error ? (
            <div className="flex h-full flex-col items-center justify-center p-8 text-center">
              <AlertCircle className="w-10 h-10 text-red-500 mb-3" />
              <p className="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-1">{error}</p>
              <p className="text-xs text-gray-500 mb-4">该文件可能需要下载到本地应用中查看</p>
              <button
                type="button"
                onClick={handleDownload}
                className="inline-flex items-center gap-2 px-4 py-2 bg-primary-600 hover:bg-primary-700 text-white text-xs font-medium rounded-lg transition-colors shadow-sm"
              >
                <Download className="w-3.5 h-3.5" />
                下载查看
              </button>
            </div>
          ) : blobUrl ? (
            kind === 'markdown' ? (
              <MarkdownViewer
                url={blobUrl}
                fileName={file.name}
                fileId={file.id}
                companyId={file.companyId}
                parentPath={file.parentPath}
                isDark={isDark}
                initialMode={file.initialMode}
                onSaved={onSaved}
                onDirtyChange={setIsMarkdownDirty}
              />
            ) : kind === 'docx' ? (
              <DocxViewer url={blobUrl} fileName={file.name} isDark={isDark} />
            ) : kind === 'pptx' ? (
              <PptxViewer url={blobUrl} fileName={file.name} isDark={isDark} />
            ) : kind === 'xlsx' ? (
              <XlsxViewer url={blobUrl} fileName={file.name} isDark={isDark} />
            ) : kind === 'text' ? (
              <CodeViewer ref={codeViewerRef} url={blobUrl} fileName={file.name} isDark={isDark} />
            ) : kind === 'pdf' ? (
              <iframe src={blobUrl} className="w-full h-full border-0 bg-white" title={file.name} />
            ) : kind === 'drawio' ? (
              <DrawioViewer url={blobUrl} fileName={file.name} isDark={isDark} />
            ) : kind === 'image' ? (
              <div
                className="relative flex h-full w-full items-center justify-center p-4 overflow-hidden select-none bg-gray-950/5 dark:bg-black/30"
                onWheel={(e) => {
                  if (e.ctrlKey || e.metaKey || Math.abs(e.deltaY) > 20) {
                    e.preventDefault();
                    if (e.deltaY < 0) handleZoomIn();
                    else handleZoomOut();
                  }
                }}
              >
                {/* Floating Left Prev Button */}
                {hasPrev && onPrev && (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      onPrev();
                    }}
                    className="absolute left-3 md:left-5 top-1/2 -translate-y-1/2 z-20 flex h-11 w-11 items-center justify-center rounded-full bg-black/45 hover:bg-black/75 text-white backdrop-blur-md shadow-xl border border-white/20 transition-all hover:scale-110 active:scale-95"
                    title="上一张 (←)"
                    aria-label="Previous Image"
                  >
                    <ChevronLeft className="w-6 h-6" />
                  </button>
                )}

                {/* Floating Right Next Button */}
                {hasNext && onNext && (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      onNext();
                    }}
                    className="absolute right-3 md:right-5 top-1/2 -translate-y-1/2 z-20 flex h-11 w-11 items-center justify-center rounded-full bg-black/45 hover:bg-black/75 text-white backdrop-blur-md shadow-xl border border-white/20 transition-all hover:scale-110 active:scale-95"
                    title="下一张 (→)"
                    aria-label="Next Image"
                  >
                    <ChevronRight className="w-6 h-6" />
                  </button>
                )}

                {/* Image Container with Zoom & Rotation */}
                <div
                  className="flex h-full w-full items-center justify-center overflow-auto"
                  onDoubleClick={() => {
                    setScale((prev) => (prev > 1.1 ? 1 : 1.75));
                  }}
                >
                  <img
                    src={blobUrl}
                    alt={file.name}
                    style={{
                      transform: `scale(${scale}) rotate(${rotation}deg)`,
                      transition: 'transform 0.15s cubic-bezier(0.2, 0, 0, 1)',
                    }}
                    className="max-h-full max-w-full object-contain rounded-lg shadow-md cursor-grab active:cursor-grabbing pointer-events-auto"
                    draggable={false}
                  />
                </div>

                {/* Bottom Floating Control Pill Toolbar */}
                <div className="absolute bottom-4 left-1/2 -translate-x-1/2 z-20 flex items-center gap-1.5 px-3 py-1.5 rounded-full bg-black/70 dark:bg-gray-900/90 text-white backdrop-blur-md shadow-2xl border border-white/15 text-xs">
                  {/* Prev */}
                  <button
                    type="button"
                    onClick={onPrev}
                    disabled={!hasPrev}
                    className={clsx(
                      "p-1.5 rounded-full transition-colors",
                      hasPrev ? "hover:bg-white/20 text-white cursor-pointer" : "text-white/30 cursor-not-allowed"
                    )}
                    title="上一张 (←)"
                  >
                    <ChevronLeft className="w-4 h-4" />
                  </button>

                  {/* Counter */}
                  {typeof itemCount === 'number' && itemCount > 0 && (
                    <span className="px-1 text-[11px] font-medium text-white/90 tabular-nums">
                      {(itemIndex ?? 0) + 1} / {itemCount}
                    </span>
                  )}

                  {/* Next */}
                  <button
                    type="button"
                    onClick={onNext}
                    disabled={!hasNext}
                    className={clsx(
                      "p-1.5 rounded-full transition-colors",
                      hasNext ? "hover:bg-white/20 text-white cursor-pointer" : "text-white/30 cursor-not-allowed"
                    )}
                    title="下一张 (→)"
                  >
                    <ChevronRight className="w-4 h-4" />
                  </button>

                  <div className="w-px h-3.5 bg-white/20 mx-0.5" />

                  {/* Zoom Out */}
                  <button
                    type="button"
                    onClick={handleZoomOut}
                    className="p-1.5 rounded-full hover:bg-white/20 text-white transition-colors"
                    title="缩小 (-)"
                  >
                    <ZoomOut className="w-4 h-4" />
                  </button>

                  {/* Scale Percent & Reset */}
                  <button
                    type="button"
                    onClick={handleResetZoom}
                    className="px-1.5 py-0.5 rounded text-[11px] font-medium text-white/90 hover:bg-white/20 transition-colors"
                    title="点击还原 100%"
                  >
                    {Math.round(scale * 100)}%
                  </button>

                  {/* Zoom In */}
                  <button
                    type="button"
                    onClick={handleZoomIn}
                    className="p-1.5 rounded-full hover:bg-white/20 text-white transition-colors"
                    title="放大 (+)"
                  >
                    <ZoomIn className="w-4 h-4" />
                  </button>

                  <div className="w-px h-3.5 bg-white/20 mx-0.5" />

                  {/* Rotate CCW */}
                  <button
                    type="button"
                    onClick={handleRotateCcw}
                    className="p-1.5 rounded-full hover:bg-white/20 text-white transition-colors"
                    title="逆时针旋转 90°"
                  >
                    <RotateCcw className="w-4 h-4" />
                  </button>

                  {/* Rotate CW */}
                  <button
                    type="button"
                    onClick={handleRotateCw}
                    className="p-1.5 rounded-full hover:bg-white/20 text-white transition-colors"
                    title="顺时针旋转 90° (R)"
                  >
                    <RotateCw className="w-4 h-4" />
                  </button>

                  <div className="w-px h-3.5 bg-white/20 mx-0.5" />

                  {/* Download */}
                  <button
                    type="button"
                    onClick={handleDownload}
                    className="p-1.5 rounded-full hover:bg-white/20 text-white transition-colors"
                    title="下载图片"
                  >
                    <Download className="w-4 h-4" />
                  </button>
                </div>
              </div>
            ) : kind === 'video' ? (
              <div className="flex h-full w-full items-center justify-center p-4 bg-black">
                <video src={blobUrl} controls className="max-h-full max-w-full rounded-lg" />
              </div>
            ) : kind === 'audio' ? (
              <div className="flex h-full w-full items-center justify-center p-8">
                <div className="w-full max-w-md p-6 bg-white dark:bg-gray-800 rounded-xl shadow-lg text-center">
                  <p className="text-sm font-medium text-gray-800 dark:text-gray-200 mb-4">{file.name}</p>
                  <audio src={blobUrl} controls className="w-full" />
                </div>
              </div>
            ) : (
              <div className="flex h-full flex-col items-center justify-center p-8 text-center">
                <div className="p-4 bg-gray-100 dark:bg-gray-700/50 rounded-2xl mb-4">
                  <FileText className="w-12 h-12 text-gray-400" />
                </div>
                <h4 className="text-sm font-semibold text-gray-900 dark:text-gray-100 mb-1">{file.name}</h4>
                <p className="text-xs text-gray-500 max-w-xs mb-5">
                  当前文件格式暂不支持在线直接渲染，您可以通过下方按钮直接下载查看。
                </p>
                <div className="flex items-center gap-3">
                  <button
                    type="button"
                    onClick={handleDownload}
                    className="inline-flex items-center gap-2 px-4 py-2 bg-primary-600 hover:bg-primary-700 text-white text-xs font-medium rounded-lg transition-colors shadow-sm"
                  >
                    <Download className="w-3.5 h-3.5" />
                    下载文件
                  </button>
                  <button
                    type="button"
                    onClick={handleOpenInNewTab}
                    className="inline-flex items-center gap-2 px-4 py-2 bg-white dark:bg-gray-700 border border-gray-200 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600 text-gray-700 dark:text-gray-200 text-xs font-medium rounded-lg transition-colors"
                  >
                    <ExternalLink className="w-3.5 h-3.5" />
                    新标签页打开
                  </button>
                </div>
              </div>
            )
          ) : null}
          </div>

          {/* Right Comments Sidebar Drawer */}
          {isCommentsOpen && file.id && file.companyId && (
            <div className="w-80 sm:w-96 border-l border-gray-200/80 dark:border-gray-800 bg-white dark:bg-gray-900 flex flex-col h-full shrink-0 z-30 animate-in slide-in-from-right-4 duration-150">
              <FileCommentsPanel
                fileId={file.id}
                companyId={file.companyId}
                variant="sidebar"
                onClose={() => setIsCommentsOpen(false)}
                onCountChange={setCommentCount}
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
