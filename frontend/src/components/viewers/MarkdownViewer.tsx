import React, { useState, useEffect, useMemo, useRef } from 'react';
import { marked } from 'marked';
import {
  FileText,
  Code2,
  Columns,
  Copy,
  Check,
  Loader2,
  AlertCircle,
  Hash,
  AlignLeft,
} from 'lucide-react';
import clsx from 'clsx';
import { CodeViewer, CodeViewerHandle } from './CodeViewer';

interface MarkdownViewerProps {
  url: string;
  fileName: string;
  isDark?: boolean;
}

type ViewMode = 'rendered' | 'source' | 'split';

export function MarkdownViewer({ url, fileName, isDark = false }: MarkdownViewerProps) {
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('rendered');
  const [copied, setCopied] = useState(false);
  const codeViewerRef = useRef<CodeViewerHandle>(null);

  useEffect(() => {
    let isMounted = true;
    setLoading(true);
    setError(null);

    fetch(url)
      .then((res) => {
        if (!res.ok) throw new Error(`加载文件失败 (${res.status})`);
        return res.text();
      })
      .then((text) => {
        if (!isMounted) return;
        setContent(text);
      })
      .catch((err) => {
        if (!isMounted) return;
        setError(err.message || '加载 Markdown 内容失败');
      })
      .finally(() => {
        if (isMounted) setLoading(false);
      });

    return () => {
      isMounted = false;
    };
  }, [url]);

  // Configure marked for GFM
  const renderedHtml = useMemo(() => {
    if (!content) return '';
    try {
      return marked.parse(content, {
        gfm: true,
        breaks: true,
      }) as string;
    } catch (e: any) {
      console.error('Failed to parse markdown:', e);
      return `<p class="text-red-500">解析 Markdown 出现错误: ${e?.message || ''}</p>`;
    }
  }, [content]);

  // Statistics
  const stats = useMemo(() => {
    if (!content) return { lines: 0, words: 0, chars: 0 };
    const lines = content.split('\n').length;
    const chars = content.length;
    const words = (content.match(/[\w-]+|[\u4e00-\u9fa5]/g) || []).length;
    return { lines, words, chars };
  }, [content]);

  const handleCopy = async () => {
    if (!content) return;
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch (err) {
      console.error('Failed to copy markdown content:', err);
    }
  };

  if (loading) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center text-gray-500">
        <Loader2 className="w-8 h-8 animate-spin text-primary-600" />
        <p className="text-sm font-medium">正在解析 Markdown 文档...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center p-8 text-center text-red-500">
        <AlertCircle className="w-10 h-10 mb-3" />
        <p className="text-sm font-semibold">{error}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full w-full bg-white dark:bg-gray-900 overflow-hidden">
      {/* Viewer Toolbar */}
      <div className="flex h-10 shrink-0 items-center justify-between border-b border-gray-200 dark:border-gray-800 px-4 bg-gray-50/80 dark:bg-gray-900/80 text-xs select-none">
        {/* View Mode Switcher */}
        <div className="flex items-center gap-1 bg-gray-200/70 dark:bg-gray-800 p-0.5 rounded-lg">
          <button
            type="button"
            onClick={() => setViewMode('rendered')}
            className={clsx(
              'flex items-center gap-1.5 px-2.5 py-1 rounded-md font-medium transition-all',
              viewMode === 'rendered'
                ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-xs'
                : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
            )}
            title="排版渲染预览"
          >
            <FileText className="w-3.5 h-3.5 text-indigo-500" />
            <span>预览</span>
          </button>
          <button
            type="button"
            onClick={() => setViewMode('source')}
            className={clsx(
              'flex items-center gap-1.5 px-2.5 py-1 rounded-md font-medium transition-all',
              viewMode === 'source'
                ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-xs'
                : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
            )}
            title="Markdown 源码"
          >
            <Code2 className="w-3.5 h-3.5 text-blue-500" />
            <span>源码</span>
          </button>
          <button
            type="button"
            onClick={() => setViewMode('split')}
            className={clsx(
              'hidden sm:flex items-center gap-1.5 px-2.5 py-1 rounded-md font-medium transition-all',
              viewMode === 'split'
                ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-xs'
                : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
            )}
            title="左右分屏对照"
          >
            <Columns className="w-3.5 h-3.5 text-emerald-500" />
            <span>分屏</span>
          </button>
        </div>

        {/* Right Tools: Stats & Copy */}
        <div className="flex items-center gap-3 text-gray-500 dark:text-gray-400">
          <div className="hidden md:flex items-center gap-3 text-[11px]">
            <span className="flex items-center gap-1">
              <Hash className="w-3 h-3 text-gray-400" />
              {stats.lines} 行
            </span>
            <span className="flex items-center gap-1">
              <AlignLeft className="w-3 h-3 text-gray-400" />
              {stats.words} 字
            </span>
          </div>

          <button
            type="button"
            onClick={handleCopy}
            className="flex items-center gap-1.5 px-2 py-1 rounded border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300 transition-colors"
            title="复制 Markdown 原文"
          >
            {copied ? (
              <>
                <Check className="w-3.5 h-3.5 text-green-500" />
                <span className="text-green-600 dark:text-green-400">已复制</span>
              </>
            ) : (
              <>
                <Copy className="w-3.5 h-3.5" />
                <span>复制内容</span>
              </>
            )}
          </button>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="relative flex-1 min-h-0 w-full overflow-hidden">
        {/* Rendered Mode */}
        {viewMode === 'rendered' && (
          <div className="h-full w-full overflow-y-auto px-6 py-8 md:px-12 lg:px-16 scroll-smooth bg-white dark:bg-gray-900">
            <article
              className={clsx(
                'markdown-body mx-auto max-w-4xl text-gray-800 dark:text-gray-200',
                isDark && 'markdown-body-dark'
              )}
              dangerouslySetInnerHTML={{ __html: renderedHtml }}
            />
          </div>
        )}

        {/* Source Code Mode */}
        {viewMode === 'source' && (
          <div className="h-full w-full overflow-hidden">
            <CodeViewer ref={codeViewerRef} url={url} fileName={fileName} isDark={isDark} />
          </div>
        )}

        {/* Split Mode */}
        {viewMode === 'split' && (
          <div className="grid grid-cols-2 h-full w-full divide-x divide-gray-200 dark:divide-gray-800">
            <div className="h-full w-full overflow-hidden">
              <CodeViewer ref={codeViewerRef} url={url} fileName={fileName} isDark={isDark} />
            </div>
            <div className="h-full w-full overflow-y-auto px-6 py-6 scroll-smooth bg-white dark:bg-gray-900">
              <article
                className={clsx(
                  'markdown-body text-gray-800 dark:text-gray-200 text-sm',
                  isDark && 'markdown-body-dark'
                )}
                dangerouslySetInnerHTML={{ __html: renderedHtml }}
              />
            </div>
          </div>
        )}
      </div>

      {/* Scoped CSS for Rich Markdown Typography */}
      <style>{`
        .markdown-body {
          font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans", Helvetica, Arial, sans-serif;
          font-size: 14px;
          line-height: 1.6;
          word-wrap: break-word;
        }
        .markdown-body h1 {
          font-size: 1.85em;
          font-weight: 700;
          padding-bottom: 0.3em;
          border-bottom: 1px solid #e5e7eb;
          margin-top: 1.2em;
          margin-bottom: 0.6em;
        }
        .markdown-body h2 {
          font-size: 1.45em;
          font-weight: 600;
          padding-bottom: 0.25em;
          border-bottom: 1px solid #e5e7eb;
          margin-top: 1.1em;
          margin-bottom: 0.5em;
        }
        .markdown-body h3 {
          font-size: 1.2em;
          font-weight: 600;
          margin-top: 1em;
          margin-bottom: 0.4em;
        }
        .markdown-body h4, .markdown-body h5, .markdown-body h6 {
          font-size: 1em;
          font-weight: 600;
          margin-top: 0.8em;
          margin-bottom: 0.4em;
        }
        .markdown-body p, .markdown-body ul, .markdown-body ol {
          margin-top: 0;
          margin-bottom: 0.9em;
        }
        .markdown-body ul {
          list-style-type: disc;
          padding-left: 1.8em;
        }
        .markdown-body ol {
          list-style-type: decimal;
          padding-left: 1.8em;
        }
        .markdown-body li {
          margin-top: 0.25em;
        }
        .markdown-body blockquote {
          margin: 0.8em 0;
          padding: 0.6em 1em;
          color: #4b5563;
          border-left: 4px solid #6366f1;
          background-color: #f8fafc;
          border-radius: 0 8px 8px 0;
        }
        .markdown-body hr {
          height: 1px;
          padding: 0;
          margin: 1.5em 0;
          background-color: #e5e7eb;
          border: 0;
        }
        .markdown-body pre {
          padding: 1em;
          overflow: auto;
          font-size: 85%;
          line-height: 1.45;
          background-color: #1e293b;
          color: #f8fafc;
          border-radius: 8px;
          margin-top: 0.5em;
          margin-bottom: 1em;
        }
        .markdown-body code {
          font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace;
          padding: 0.2em 0.4em;
          margin: 0;
          font-size: 85%;
          background-color: rgba(175, 184, 193, 0.2);
          border-radius: 6px;
        }
        .markdown-body pre code {
          padding: 0;
          background-color: transparent;
          color: inherit;
        }
        .markdown-body table {
          border-spacing: 0;
          border-collapse: collapse;
          margin-top: 0.5em;
          margin-bottom: 1em;
          width: 100%;
          overflow: auto;
          display: block;
        }
        .markdown-body table th,
        .markdown-body table td {
          padding: 6px 13px;
          border: 1px solid #d1d5db;
        }
        .markdown-body table tr:nth-child(2n) {
          background-color: #f9fafb;
        }
        .markdown-body table th {
          font-weight: 600;
          background-color: #f3f4f6;
        }
        .markdown-body a {
          color: #4f46e5;
          text-decoration: underline;
          text-underline-offset: 2px;
        }
        .markdown-body img {
          max-width: 100%;
          box-sizing: content-box;
          border-radius: 8px;
          margin: 0.5em 0;
        }

        /* Dark Mode Adjustments */
        .markdown-body-dark h1,
        .markdown-body-dark h2 {
          border-bottom-color: #374151;
        }
        .markdown-body-dark blockquote {
          color: #9ca3af;
          background-color: #111827;
          border-left-color: #818cf8;
        }
        .markdown-body-dark hr {
          background-color: #374151;
        }
        .markdown-body-dark pre {
          background-color: #0f172a;
        }
        .markdown-body-dark table th,
        .markdown-body-dark table td {
          border-color: #374151;
        }
        .markdown-body-dark table th {
          background-color: #1f2937;
        }
        .markdown-body-dark table tr:nth-child(2n) {
          background-color: #111827;
        }
        .markdown-body-dark a {
          color: #818cf8;
        }
      `}</style>
    </div>
  );
}
