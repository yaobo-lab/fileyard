import React, { useState, useEffect, useRef } from 'react';
import * as docx from 'docx-preview';
import { Loader2 } from 'lucide-react';

interface DocxViewerProps {
  url: string;
  fileName: string;
  isDark?: boolean;
}

export function DocxViewer({ url, fileName, isDark = false }: DocxViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);

    const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
    fetch(url, {
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    })
      .then((res) => {
        if (!res.ok) throw new Error(`下载文档失败 (${res.status})`);
        return res.blob();
      })
      .then(async (blob) => {
        if (cancelled || !containerRef.current) return;
        
        containerRef.current.innerHTML = '';
        try {
          await docx.renderAsync(blob, containerRef.current, undefined, {
            className: 'docx-rendered-document',
            inWrapper: true,
            ignoreWidth: false,
            ignoreHeight: false,
            breakPages: true,
            useBase64URL: true,
            renderHeaders: true,
            renderFooters: true,
            renderFootnotes: true,
            renderEndnotes: true,
          });
          if (!cancelled) {
            setLoading(false);
          }
        } catch (parseErr: any) {
          if (!cancelled) {
            setError(parseErr.message || 'Word 文档解析失败，请尝试下载后查看');
            setLoading(false);
          }
        }
      })
      .catch((err) => {
        if (cancelled) return;
        setError(err.message || 'Word 文档下载失败');
        setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [url, fileName]);

  return (
    <div className={`h-full w-full overflow-auto p-4 flex flex-col items-center select-text ${isDark ? 'bg-gray-900' : 'bg-gray-100'}`}>
      {error && (
        <div className="flex h-full flex-col items-center justify-center p-8 text-center my-auto">
          <p className="text-sm font-medium text-red-500 mb-2">{error}</p>
          <p className="text-xs text-gray-500">建议点击右上角下载按钮查看原始 Word 文档</p>
        </div>
      )}

      {loading && !error && (
        <div className="flex h-full items-center justify-center gap-2 my-auto">
          <Loader2 className="w-6 h-6 animate-spin text-primary-600" />
          <span className="text-sm text-gray-500">正在解析 Word 文档排版与内容...</span>
        </div>
      )}

      <div
        ref={containerRef}
        className={`w-full max-w-4xl flex flex-col items-center ${loading || error ? 'hidden' : 'block'}`}
      />
    </div>
  );
}


