import React, { useState, useEffect, useRef } from 'react';
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
} from 'lucide-react';
import clsx from 'clsx';
import { CodeViewer, CodeViewerHandle } from './viewers/CodeViewer';
import { DocxViewer } from './viewers/DocxViewer';
import { XlsxViewer } from './viewers/XlsxViewer';
import { PptxViewer } from './viewers/PptxViewer';
import { DrawioViewer } from './viewers/DrawioViewer';
import { FileGlyphVisual } from './FileGlyphs';

interface FilePreviewModalProps {
  isOpen: boolean;
  onClose: () => void;
  file: {
    id?: string;
    companyId?: string;
    name: string;
    url: string;
    type?: string;
    size?: number | string;
  } | null;
}

export type SupportedKind =
  | 'text'
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
    'txt', 'text', 'md', 'mdx', 'markdown', 'json', 'jsonc', 'json5', 'csv', 'tsv',
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
  text: 'w-[min(96vw,84rem)] h-[86vh]',
  pdf: 'w-[min(96vw,72rem)] h-[88vh]',
  docx: 'w-[min(96vw,72rem)] h-[88vh]',
  pptx: 'w-[min(96vw,84rem)] h-[88vh]',
  xlsx: 'w-[min(96vw,98rem)] h-[88vh]',
  drawio: 'w-[min(96vw,86rem)] h-[88vh]',
  image: 'w-auto max-w-[min(96vw,72rem)] max-h-[88vh]',
  video: 'w-[min(96vw,72rem)] h-auto max-h-[88vh]',
  audio: 'w-[min(96vw,36rem)] h-auto',
  other: 'w-[min(96vw,36rem)] h-auto',
};

export function FilePreviewModal({ isOpen, onClose, file }: FilePreviewModalProps) {
  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const codeViewerRef = useRef<CodeViewerHandle>(null);

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

  // Esc to close
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen || !file) return null;

  const kind = detectFileKind(file.name, file.type);
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
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-3 sm:p-5 bg-black/75 backdrop-blur-xs transition-opacity animate-in fade-in duration-100">
      <div
        className={clsx(
          'relative flex flex-col bg-white dark:bg-gray-800 rounded-2xl shadow-2xl border border-gray-100 dark:border-gray-700/80 overflow-hidden transition-all duration-150',
          isFullscreen ? 'w-full h-full rounded-none' : MODAL_SIZES[kind]
        )}
      >
        {/* Top Header Bar */}
        <div className="flex h-12 shrink-0 items-center justify-between border-b border-gray-100 dark:border-gray-700/80 px-4 bg-gray-50/70 dark:bg-gray-900/50 backdrop-blur-xs">
          <div className="flex items-center gap-2.5 min-w-0 pr-3">
            <div className="shrink-0 flex items-center justify-center">
              <FileGlyphVisual file={{ id: file.id || 'preview', name: file.name, type: file.type }} className="h-6 w-auto" />
            </div>
            <h3 className="text-sm font-semibold text-gray-900 dark:text-gray-100 truncate" title={file.name}>
              {file.name}
            </h3>
          </div>

          <div className="flex items-center gap-1 shrink-0">
            {/* Search (for code/text) */}
            {kind === 'text' && (
              <button
                type="button"
                onClick={() => codeViewerRef.current?.toggleSearch()}
                className="p-1.5 rounded-lg text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors"
                title="搜索 (Ctrl+F)"
              >
                <Search className="w-4 h-4" />
              </button>
            )}

            {/* Open in new tab */}
            {blobUrl && (
              <button
                type="button"
                onClick={handleOpenInNewTab}
                className="p-1.5 rounded-lg text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors"
                title="在新标签页中打开"
              >
                <ExternalLink className="w-4 h-4" />
              </button>
            )}

            {/* Download */}
            <button
              type="button"
              onClick={handleDownload}
              className="p-1.5 rounded-lg text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors"
              title="下载文件"
            >
              <Download className="w-4 h-4" />
            </button>

            {/* Toggle Fullscreen */}
            {!['audio', 'other'].includes(kind) && (
              <button
                type="button"
                onClick={() => setIsFullscreen(!isFullscreen)}
                className="p-1.5 rounded-lg text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors"
                title={isFullscreen ? '还原窗口' : '全屏显示'}
              >
                {isFullscreen ? <Minimize2 className="w-4 h-4" /> : <Maximize2 className="w-4 h-4" />}
              </button>
            )}

            {/* Close */}
            <button
              type="button"
              onClick={onClose}
              className="p-1.5 rounded-lg text-gray-500 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white hover:bg-gray-100 dark:hover:bg-gray-700/60 transition-colors ml-1"
              title="关闭 (Esc)"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* Content Viewer Body */}
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
            kind === 'docx' ? (
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
              <div className="flex h-full w-full items-center justify-center p-4 overflow-auto">
                <img src={blobUrl} alt={file.name} className="max-h-full max-w-full object-contain rounded-lg shadow-sm" />
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
      </div>
    </div>
  );
}
