import React, { useState, useEffect } from 'react';
import { ReactDocxViewer } from '@extend-ai/react-docx';
import { Loader2 } from 'lucide-react';

interface DocxViewerProps {
  url: string;
  fileName: string;
  isDark?: boolean;
}

export function DocxViewer({ url, fileName, isDark = false }: DocxViewerProps) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [fileBuffer, setFileBuffer] = useState<ArrayBuffer | undefined>(undefined);

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
        return res.arrayBuffer();
      })
      .then((buffer) => {
        if (cancelled) return;
        setFileBuffer(buffer);
        setLoading(false);
      })
      .catch((err) => {
        if (cancelled) return;
        setError(err.message || 'Word 文档解析失败，请尝试下载后查看');
        setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [url, fileName]);

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center p-8 text-center">
        <p className="text-sm font-medium text-red-500 mb-2">{error}</p>
        <p className="text-xs text-gray-500">建议点击右上角下载按钮查看原始 Word 文档</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center gap-2">
        <Loader2 className="w-6 h-6 animate-spin text-primary-600" />
        <span className="text-sm text-gray-500">正在解析 Word 文档，请稍候...</span>
      </div>
    );
  }

  return (
    <div className={`h-full w-full overflow-auto p-4 flex justify-center ${isDark ? 'bg-gray-900' : 'bg-gray-100'} select-text`}>
      <ReactDocxViewer
        file={fileBuffer}
        className="max-w-4xl w-full shadow-lg rounded-lg overflow-hidden my-auto"
        emptyState={<span className="text-gray-400 text-sm">未能加载文档内容</span>}
      />
    </div>
  );
}

