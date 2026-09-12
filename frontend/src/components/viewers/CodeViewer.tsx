import React, { useEffect, useRef, useState, forwardRef, useImperativeHandle } from 'react';
import { EditorState, type Extension } from '@codemirror/state';
import { EditorView, basicSetup } from 'codemirror';
import { LanguageDescription } from '@codemirror/language';
import { languages } from '@codemirror/language-data';
import { search, openSearchPanel, closeSearchPanel, searchPanelOpen } from '@codemirror/search';
import { githubLight, githubDark } from '@uiw/codemirror-theme-github';
import { Loader2 } from 'lucide-react';

const MAX_BYTES = 5_000_000;

const CODEMIRROR_PHRASES: Record<string, string> = {
  Find: '查找',
  Replace: '替换',
  next: '下一个',
  previous: '上一个',
  all: '全部',
  'match case': '区分大小写',
  'by word': '全字匹配',
  regexp: '正则表达式',
  replace: '替换',
  'replace all': '全部替换',
  close: '关闭',
  'current match': '当前匹配',
  'on line': '在行',
  'Go to line': '跳转到行',
  go: '跳转',
};

const layoutTheme = EditorView.theme({
  '&': { height: '100%' },
  '.cm-scroller': {
    fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
    fontSize: '13px',
    lineHeight: '1.6',
  },
  '.cm-panels-top': {
    borderBottom: '1px solid var(--color-border, #e5e7eb)',
  },
  '.cm-panel.cm-search': {
    display: 'flex',
    flexWrap: 'wrap',
    alignItems: 'center',
    gap: '6px',
    padding: '8px 12px',
    backgroundColor: 'var(--color-bg, #ffffff)',
    color: 'inherit',
  },
  '.cm-panel.cm-search .cm-textfield': {
    boxSizing: 'border-box',
    width: 'min(240px, 40vw)',
    height: '28px',
    padding: '0 8px',
    border: '1px solid #d1d5db',
    borderRadius: '6px',
    fontSize: '12px',
  },
  '.cm-panel.cm-search .cm-button': {
    boxSizing: 'border-box',
    height: '28px',
    padding: '0 8px',
    border: '1px solid #d1d5db',
    borderRadius: '6px',
    background: '#f3f4f6',
    cursor: 'pointer',
    fontSize: '12px',
  },
});

async function getLanguageExtension(fileName: string): Promise<Extension[]> {
  const description = LanguageDescription.matchFilename(languages, fileName);
  if (!description) return [];
  try {
    const support = await description.load();
    return [support];
  } catch {
    return [];
  }
}

export interface CodeViewerHandle {
  toggleSearch: () => void;
}

interface CodeViewerProps {
  url: string;
  fileName: string;
  isDark?: boolean;
}

export const CodeViewer = forwardRef<CodeViewerHandle, CodeViewerProps>(function CodeViewer(
  { url, fileName, isDark = false },
  ref
) {
  const [text, setText] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const hostRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);

  useImperativeHandle(ref, () => ({
    toggleSearch: () => {
      const view = viewRef.current;
      if (!view) return;
      if (searchPanelOpen(view.state)) closeSearchPanel(view);
      else openSearchPanel(view);
    },
  }));

  useEffect(() => {
    let cancelled = false;
    setText(null);
    setError(null);

    const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
    fetch(url, {
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    })
      .then((res) => {
        if (!res.ok) throw new Error(`加载失败 (${res.status})`);
        const size = Number(res.headers.get('content-length') ?? '0');
        if (size > MAX_BYTES) {
          throw new Error('文件体积过大，暂不支持在线完整预览，请下载后查看');
        }
        return res.text();
      })
      .then((body) => {
        if (!cancelled) setText(body);
      })
      .catch((err) => {
        if (!cancelled) setError(err.message || '文本加载失败');
      });

    return () => {
      cancelled = true;
    };
  }, [url]);

  useEffect(() => {
    if (text === null || !hostRef.current) return;

    let cancelled = false;
    const host = hostRef.current;

    void (async () => {
      const langExt = await getLanguageExtension(fileName);
      if (cancelled) return;

      viewRef.current?.destroy();
      viewRef.current = new EditorView({
        parent: host,
        state: EditorState.create({
          doc: text,
          extensions: [
            basicSetup,
            search({ top: true }),
            EditorState.phrases.of(CODEMIRROR_PHRASES),
            isDark ? githubDark : githubLight,
            layoutTheme,
            EditorState.readOnly.of(true),
            EditorView.lineWrapping,
            ...langExt,
          ],
        }),
      });
    })();

    return () => {
      cancelled = true;
      viewRef.current?.destroy();
      viewRef.current = null;
    };
  }, [text, isDark, fileName]);

  if (error) {
    return (
      <div className="flex h-full items-center justify-center p-6 text-center text-sm text-red-500">
        {error}
      </div>
    );
  }

  if (text === null) {
    return (
      <div className="flex h-full items-center justify-center gap-2">
        <Loader2 className="w-6 h-6 animate-spin text-primary-600" />
        <span className="text-sm text-gray-500">正在载入代码与文档...</span>
      </div>
    );
  }

  return <div ref={hostRef} className="h-full w-full overflow-hidden" />;
});
