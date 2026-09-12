import React, { useState, useEffect } from 'react';
import { XlsxViewer as ReactXlsxViewer, XlsxViewerProvider, useXlsxViewerController } from '@extend-ai/react-xlsx';
import { Loader2 } from 'lucide-react';

interface XlsxViewerProps {
  url: string;
  fileName: string;
  isDark?: boolean;
}

function InnerXlsxViewer({
  buffer,
  fileName,
  isDark,
}: {
  buffer: ArrayBuffer;
  fileName: string;
  isDark: boolean;
}) {
  const controller = useXlsxViewerController(
    React.useMemo(
      () => ({
        allowResizeInReadOnly: true,
        file: buffer,
        fileName,
        readOnly: true,
        useWorker: false,
      }),
      [fileName, buffer]
    )
  );

  return (
    <XlsxViewerProvider controller={controller} isDark={isDark}>
      <div className="h-full w-full overflow-hidden flex flex-col">
        <ReactXlsxViewer
          experimentalCanvas
          allowResizeInReadOnly
          className="h-full min-h-0 min-w-0"
          height="100%"
          isDark={isDark}
          readOnly
          rounded={false}
          showDefaultToolbar={true}
        />
      </div>
    </XlsxViewerProvider>
  );
}

export function XlsxViewer({ url, fileName, isDark = false }: XlsxViewerProps) {
  const [buffer, setBuffer] = useState<ArrayBuffer | null>(null);
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
        if (!res.ok) throw new Error(`加载表格失败 (${res.status})`);
        return res.arrayBuffer();
      })
      .then((buf) => {
        if (!cancelled) {
          setBuffer(buf);
          setLoading(false);
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err.message || 'Excel 解析失败');
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [url]);

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center p-8 text-center">
        <p className="text-sm font-medium text-red-500 mb-2">{error}</p>
        <p className="text-xs text-gray-500">建议下载后使用 Excel 查看该工作表</p>
      </div>
    );
  }

  if (loading || !buffer) {
    return (
      <div className="flex h-full items-center justify-center gap-2">
        <Loader2 className="w-6 h-6 animate-spin text-primary-600" />
        <span className="text-sm text-gray-500">正在解析 Excel 表格...</span>
      </div>
    );
  }

  return <InnerXlsxViewer buffer={buffer} fileName={fileName} isDark={isDark} />;
}
