import React, { useState, useEffect, useMemo, useRef, useCallback } from 'react';
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
  Save,
  Bold,
  Italic,
  Heading,
  Quote,
  Code,
  List,
  ListOrdered,
  CheckSquare,
  Table,
  Minus,
  Link as LinkIcon,
  Image as ImageIcon,
  RotateCcw,
} from 'lucide-react';
import clsx from 'clsx';
import { EditorState, type Extension } from '@codemirror/state';
import { EditorView, basicSetup } from 'codemirror';
import { LanguageDescription } from '@codemirror/language';
import { languages } from '@codemirror/language-data';
import { search } from '@codemirror/search';
import { githubLight, githubDark } from '@uiw/codemirror-theme-github';
import { useTranslations } from '../../context/I18nContext';
import { useModalDialog } from '../../context/ModalDialogContext';
import { copyToClipboard } from '@/lib/utils';

export interface MarkdownViewerProps {
  url: string;
  fileName: string;
  fileId?: string;
  companyId?: string;
  parentPath?: string;
  isDark?: boolean;
  initialMode?: 'rendered' | 'source' | 'split';
  onSaved?: (newMetadata?: any) => void;
  onDirtyChange?: (isDirty: boolean) => void;
}

type ViewMode = 'rendered' | 'source' | 'split';

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
  '&': { height: '100%', fontSize: '13px' },
  '.cm-scroller': {
    fontFamily: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", monospace',
    lineHeight: '1.6',
    padding: '12px 0',
  },
  '.cm-content': {
    padding: '0 16px',
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

async function getMarkdownLanguageExtension(): Promise<Extension[]> {
  const description = LanguageDescription.matchFilename(languages, 'file.md');
  if (!description) return [];
  try {
    const support = await description.load();
    return [support];
  } catch {
    return [];
  }
}

export function MarkdownViewer({
  url,
  fileName,
  fileId,
  companyId,
  parentPath,
  isDark = false,
  initialMode,
  onSaved,
  onDirtyChange,
}: MarkdownViewerProps) {
  const t = useTranslations('MarkdownEditor');
  const { confirm: modalConfirm } = useModalDialog();
  const [content, setContent] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>(initialMode || 'rendered');
  const [copied, setCopied] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [isDirty, setIsDirty] = useState(false);
  const [isUploadingImage, setIsUploadingImage] = useState(false);

  const initialContentRef = useRef<string>('');
  const contentRef = useRef<string>('');
  const editorHostRef = useRef<HTMLDivElement>(null);
  const editorViewRef = useRef<EditorView | null>(null);
  const imageInputRef = useRef<HTMLInputElement>(null);
  const renderedContainerRef = useRef<HTMLDivElement>(null);
  const splitRenderedContainerRef = useRef<HTMLDivElement>(null);
  const onDirtyChangeRef = useRef(onDirtyChange);
  onDirtyChangeRef.current = onDirtyChange;

  // Sync ref with state
  useEffect(() => {
    contentRef.current = content ?? '';
  }, [content]);

  // Initial load
  useEffect(() => {
    let isMounted = true;
    setLoading(true);
    setError(null);

    const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
    fetch(url, {
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    })
      .then((res) => {
        if (!res.ok) throw new Error(`加载文件失败 (${res.status})`);
        return res.text();
      })
      .then((text) => {
        if (!isMounted) return;
        setContent(text);
        initialContentRef.current = text;
        setIsDirty(false);
        onDirtyChangeRef.current?.(false);
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

  // Handle save
  const handleSave = useCallback(async () => {
    if (!companyId || !fileId) {
      setSaveError('当前未指定文件 ID 或租户信息，无法保存');
      return;
    }
    const currentText = contentRef.current;
    setSaving(true);
    setSaveError(null);

    try {
      const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
      const res = await fetch(`/api/files/${companyId}/${fileId}/content`, {
        method: 'PUT',
        headers: {
          'Content-Type': 'application/json',
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
        body: JSON.stringify({ content: currentText }),
      });

      const data = await res.json().catch(() => ({}));
      if (!res.ok) {
        throw new Error(data.message || data.error || `保存失败 (${res.status})`);
      }

      initialContentRef.current = currentText;
      setIsDirty(false);
      onDirtyChangeRef.current?.(false);
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 2500);
      onSaved?.(data.file);
    } catch (err: any) {
      console.error('Failed to save file:', err);
      setSaveError(err.message || '保存失败');
    } finally {
      setSaving(false);
    }
  }, [companyId, fileId, onSaved]);

  // Global Ctrl+S / Cmd+S shortcut
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 's') {
        e.preventDefault();
        handleSave();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleSave]);

  // Upload and insert an image file into markdown at cursor
  const uploadAndInsertImage = useCallback(
    async (file: File) => {
      if (!companyId) {
        setSaveError('无法上传图片：缺少租户ID');
        return;
      }

      setIsUploadingImage(true);
      setSaveError(null);

      const baseName = file.name || `image-${Date.now()}.png`;
      const placeholder = `![上传中: ${baseName}...](uploading-${Date.now()})`;

      // Insert placeholder at current selection or end
      const view = editorViewRef.current;
      if (view) {
        const { from, to } = view.state.selection.main;
        view.dispatch({
          changes: { from, to, insert: placeholder },
          selection: { anchor: from + placeholder.length },
        });
      } else {
        setContent((prev) => (prev ? `${prev}\n${placeholder}\n` : `${placeholder}\n`));
      }

      try {
        const formData = new FormData();
        formData.append('file', file, baseName);

        const queryParams = new URLSearchParams();
        if (parentPath) {
          queryParams.set('parent_path', parentPath);
        }
        const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
        const uploadUrl = `/api/upload/${companyId}${
          queryParams.toString() ? `?${queryParams.toString()}` : ''
        }`;

        const res = await fetch(uploadUrl, {
          method: 'POST',
          headers: token ? { Authorization: `Bearer ${token}` } : undefined,
          body: formData,
        });

        const data = await res.json().catch(() => ({}));
        if (!res.ok || data.error) {
          throw new Error(data.message || data.error || '上传图片失败');
        }

        const uploadedId = data.file_id;
        const finalName = data.file_name || baseName;
        const markdownImg = `![${finalName}](/api/download/${companyId}/${uploadedId})`;

        // Replace placeholder in editor
        if (editorViewRef.current) {
          const docText = editorViewRef.current.state.doc.toString();
          const idx = docText.indexOf(placeholder);
          if (idx !== -1) {
            editorViewRef.current.dispatch({
              changes: { from: idx, to: idx + placeholder.length, insert: markdownImg },
              selection: { anchor: idx + markdownImg.length },
            });
          }
        } else {
          setContent((prev) => (prev ? prev.replace(placeholder, markdownImg) : markdownImg));
        }

        onSaved?.(); // refresh file list in background
      } catch (err: any) {
        console.error('Failed to upload markdown image:', err);
        const failNotice = `<!-- 图片上传失败: ${err.message || '网络错误'} -->`;
        if (editorViewRef.current) {
          const docText = editorViewRef.current.state.doc.toString();
          const idx = docText.indexOf(placeholder);
          if (idx !== -1) {
            editorViewRef.current.dispatch({
              changes: { from: idx, to: idx + placeholder.length, insert: failNotice },
            });
          }
        } else {
          setContent((prev) => (prev ? prev.replace(placeholder, failNotice) : ''));
        }
        setSaveError(`上传图片失败: ${err.message || '网络错误'}`);
      } finally {
        setIsUploadingImage(false);
      }
    },
    [companyId, parentPath, onSaved]
  );

  const uploadAndInsertImageRef = useRef(uploadAndInsertImage);
  uploadAndInsertImageRef.current = uploadAndInsertImage;

  // Initialize or reconfigure CodeMirror 6 Editor
  useEffect(() => {
    if (viewMode === 'rendered' || content === null || !editorHostRef.current) {
      return;
    }

    let cancelled = false;
    const host = editorHostRef.current;

    void (async () => {
      const langExt = await getMarkdownLanguageExtension();
      if (cancelled) return;

      editorViewRef.current?.destroy();

      const updateListener = EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          const docText = update.state.doc.toString();
          setContent(docText);
          const dirty = docText !== initialContentRef.current;
          setIsDirty(dirty);
          onDirtyChangeRef.current?.(dirty);
        }
      });

      editorViewRef.current = new EditorView({
        parent: host,
        state: EditorState.create({
          doc: contentRef.current,
          extensions: [
            basicSetup,
            search({ top: true }),
            EditorState.phrases.of(CODEMIRROR_PHRASES),
            isDark ? githubDark : githubLight,
            layoutTheme,
            EditorView.lineWrapping,
            updateListener,
            ...langExt,
          ],
        }),
      });

      // Handle paste event for images (e.g. screenshots from clipboard Ctrl+V)
      const handlePaste = (e: ClipboardEvent) => {
        const items = e.clipboardData?.items;
        if (!items) return;
        for (let i = 0; i < items.length; i++) {
          const item = items[i];
          if (item.type.startsWith('image/')) {
            const file = item.getAsFile();
            if (file) {
              e.preventDefault();
              void uploadAndInsertImageRef.current?.(file);
              return;
            }
          }
        }
      };

      // Handle drop event for image files
      const handleDrop = (e: DragEvent) => {
        const files = e.dataTransfer?.files;
        if (files && files.length > 0) {
          for (let i = 0; i < files.length; i++) {
            const file = files[i];
            if (file.type.startsWith('image/')) {
              e.preventDefault();
              void uploadAndInsertImageRef.current?.(file);
              return;
            }
          }
        }
      };

      host.addEventListener('paste', handlePaste);
      host.addEventListener('drop', handleDrop);

      (host as any)._cleanupListeners = () => {
        host.removeEventListener('paste', handlePaste);
        host.removeEventListener('drop', handleDrop);
      };
    })();

    return () => {
      cancelled = true;
      if ((host as any)._cleanupListeners) {
        (host as any)._cleanupListeners();
      }
      editorViewRef.current?.destroy();
      editorViewRef.current = null;
    };
  }, [viewMode, isDark]);

  // Markdown Formatting Helper for CodeMirror
  const insertFormat = useCallback((prefix: string, suffix: string = '', defaultPlaceholder: string = '') => {
    const view = editorViewRef.current;
    if (!view) return;
    const { state, dispatch } = view;
    const { from, to } = state.selection.main;
    const selectedText = state.sliceDoc(from, to) || defaultPlaceholder;
    const replacement = `${prefix}${selectedText}${suffix}`;

    dispatch({
      changes: { from, to, insert: replacement },
      selection: {
        anchor: from + prefix.length,
        head: from + prefix.length + selectedText.length,
      },
    });
    view.focus();
  }, []);

  // Trigger file input for image upload
  const handleImageButtonClick = () => {
    imageInputRef.current?.click();
  };

  const handleImageFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) {
      void uploadAndInsertImage(file);
    }
    e.target.value = '';
  };

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

  // Authenticated and relative image resolver in rendered preview
  useEffect(() => {
    if (!content) return;
    const containers = [renderedContainerRef.current, splitRenderedContainerRef.current].filter(Boolean);
    if (containers.length === 0) return;

    const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
    const createdBlobUrls: string[] = [];

    containers.forEach((container) => {
      const images = container!.querySelectorAll<HTMLImageElement>('img');
      images.forEach(async (img) => {
        const src = img.getAttribute('src');
        if (!src) return;

        // Skip if already a blob URL or inline data URL
        if (src.startsWith('blob:') || src.startsWith('data:')) {
          return;
        }

        // 1. Direct /api/download/ or /api/preview/ URL
        if (src.startsWith('/api/download/') || src.startsWith('/api/preview/')) {
          if (!token) return;
          try {
            const res = await fetch(src, {
              headers: { Authorization: `Bearer ${token}` },
            });
            if (res.ok) {
              const blob = await res.blob();
              const blobUrl = URL.createObjectURL(blob);
              createdBlobUrls.push(blobUrl);
              img.src = blobUrl;
            }
          } catch (e) {
            console.warn('Failed to load authenticated markdown image:', src, e);
          }
          return;
        }

        // 2. Relative paths like "assets/pasted-image-..." or "image.png"
        if (!src.startsWith('http://') && !src.startsWith('https://') && !src.startsWith('/')) {
          if (companyId && token) {
            const basename = src.split('/').pop() || src;
            try {
              const listRes = await fetch(`/api/files/${companyId}`, {
                headers: { Authorization: `Bearer ${token}` },
              });
              if (listRes.ok) {
                const files = await listRes.json();
                const matched = Array.isArray(files) ? files.find((f: any) => f.name === basename) : null;
                if (matched) {
                  const downloadUrl = `/api/download/${companyId}/${matched.id}`;
                  const imgRes = await fetch(downloadUrl, {
                    headers: { Authorization: `Bearer ${token}` },
                  });
                  if (imgRes.ok) {
                    const blob = await imgRes.blob();
                    const blobUrl = URL.createObjectURL(blob);
                    createdBlobUrls.push(blobUrl);
                    img.src = blobUrl;
                    return;
                  }
                }
              }
            } catch (e) {
              console.warn('Failed to resolve relative image path:', src, e);
            }
          }

          // If not found in file database, provide an informative indicator
          img.onerror = () => {
            img.style.display = 'inline-block';
            img.style.border = '1px dashed #f59e0b';
            img.style.borderRadius = '8px';
            img.style.padding = '8px 12px';
            img.style.backgroundColor = 'rgba(245, 158, 11, 0.08)';
            img.style.color = '#d97706';
            img.style.fontSize = '12px';
            img.alt = `[图片未找到: ${src} - 请使用工具栏图片按钮上传或粘贴]`;
            img.title = `未在网盘中找到该相对路径图片: ${src}。您可以直接在编辑器中按 Ctrl+V 粘贴截图，或点击工具栏图片按钮上传。`;
          };
        }
      });
    });

    return () => {
      createdBlobUrls.forEach((url) => URL.revokeObjectURL(url));
    };
  }, [renderedHtml, viewMode, companyId]);

  // Statistics
  const stats = useMemo(() => {
    if (!content) return { lines: 0, words: 0, chars: 0 };
    const lines = content.split('\n').length;
    const chars = content.length;
    const words = (content.match(/[\w-]+|[\u4e00-\u9fa5]/g) || []).length;
    return { lines, words, chars };
  }, [content]);

  const handleCopy = async () => {
    const textToCopy = editorViewRef.current
      ? editorViewRef.current.state.doc.toString()
      : (contentRef.current || content || '');
    if (!textToCopy) return;
    try {
      const ok = await copyToClipboard(textToCopy);
      if (ok) {
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      }
    } catch (err) {
      console.error('Failed to copy markdown content:', err);
    }
  };

  const handleResetContent = async () => {
    if (!isDirty) return;
    const ok = await modalConfirm({
      title: '放弃未保存的修改',
      description: t('unsavedConfirmMessage') || '您有尚未保存的修改，确定要放弃本次修改吗？',
      variant: 'warning',
      confirmText: '确定放弃',
      cancelText: '取消',
    });
    if (!ok) return;

    setContent(initialContentRef.current);
    setIsDirty(false);
    onDirtyChangeRef.current?.(false);
    if (editorViewRef.current) {
      editorViewRef.current.dispatch({
        changes: {
          from: 0,
          to: editorViewRef.current.state.doc.length,
          insert: initialContentRef.current,
        },
      });
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

  const canSave = Boolean(fileId && companyId);

  return (
    <div className="flex flex-col h-full w-full bg-white dark:bg-gray-900 overflow-hidden">
      {/* Hidden File Input for Image Upload */}
      <input
        ref={imageInputRef}
        type="file"
        accept="image/*"
        className="hidden"
        onChange={handleImageFileChange}
      />

      {/* Save / Upload Error Banner */}
      {saveError && (
        <div className="flex items-center justify-between px-4 py-2 bg-red-50 dark:bg-red-950/40 border-b border-red-200 dark:border-red-900 text-xs text-red-700 dark:text-red-300">
          <div className="flex items-center gap-2">
            <AlertCircle className="w-4 h-4 text-red-500 shrink-0" />
            <span>{saveError}</span>
          </div>
          <button
            type="button"
            onClick={() => setSaveError(null)}
            className="text-red-500 hover:text-red-700 font-medium ml-4"
          >
            ✕
          </button>
        </div>
      )}

      {/* Uploading Image Toast */}
      {isUploadingImage && (
        <div className="flex items-center gap-2 px-4 py-1.5 bg-indigo-50 dark:bg-indigo-950/50 border-b border-indigo-200 dark:border-indigo-800 text-xs text-indigo-700 dark:text-indigo-300">
          <Loader2 className="w-3.5 h-3.5 animate-spin text-indigo-600 dark:text-indigo-400 shrink-0" />
          <span>{t('uploadingImage') || '正在上传图片到网盘并插入文档...'}</span>
        </div>
      )}

      {/* Viewer / Editor Main Toolbar */}
      <div className="flex flex-wrap h-auto min-h-10 shrink-0 items-center justify-between border-b border-gray-200 dark:border-gray-800 px-3 py-1 bg-gray-50/90 dark:bg-gray-900/90 text-xs select-none gap-2">
        {/* Left: View Mode Switcher */}
        <div className="flex items-center gap-1 bg-gray-200/70 dark:bg-gray-800 p-0.5 rounded-lg shrink-0">
          <button
            type="button"
            onClick={() => setViewMode('rendered')}
            className={clsx(
              'flex items-center gap-1.5 px-2.5 py-1 rounded-md font-medium transition-all text-xs',
              viewMode === 'rendered'
                ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-xs'
                : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
            )}
            title="排版渲染预览"
          >
            <FileText className="w-3.5 h-3.5 text-indigo-500" />
            <span>{t('preview') || '预览'}</span>
          </button>
          <button
            type="button"
            onClick={() => setViewMode('source')}
            className={clsx(
              'flex items-center gap-1.5 px-2.5 py-1 rounded-md font-medium transition-all text-xs',
              viewMode === 'source'
                ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-xs'
                : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
            )}
            title="源码编辑模式"
          >
            <Code2 className="w-3.5 h-3.5 text-blue-500" />
            <span>{t('source') || '编辑'}</span>
          </button>
          <button
            type="button"
            onClick={() => setViewMode('split')}
            className={clsx(
              'hidden sm:flex items-center gap-1.5 px-2.5 py-1 rounded-md font-medium transition-all text-xs',
              viewMode === 'split'
                ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-xs'
                : 'text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-gray-200'
            )}
            title="左右分屏实时对照"
          >
            <Columns className="w-3.5 h-3.5 text-emerald-500" />
            <span>{t('split') || '分屏'}</span>
          </button>
        </div>

        {/* Center: Formatting Toolbar (visible in source or split mode) */}
        {viewMode !== 'rendered' && (
          <div className="flex items-center gap-0.5 px-1 py-0.5 border border-gray-200 dark:border-gray-700/80 rounded-lg bg-white dark:bg-gray-800 overflow-x-auto max-w-full">
            <button
              type="button"
              onClick={() => insertFormat('**', '**', '粗体')}
              className="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 transition-colors"
              title={t('bold') || '粗体'}
            >
              <Bold className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={() => insertFormat('*', '*', '斜体')}
              className="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 transition-colors"
              title={t('italic') || '斜体'}
            >
              <Italic className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={() => insertFormat('### ', '', '标题')}
              className="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 transition-colors"
              title={t('heading') || '标题'}
            >
              <Heading className="w-3.5 h-3.5" />
            </button>

            <div className="h-3.5 w-[1px] bg-gray-200 dark:bg-gray-700 mx-0.5" />

            <button
              type="button"
              onClick={() => insertFormat('> ', '', '引用')}
              className="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 transition-colors"
              title={t('quote') || '引用'}
            >
              <Quote className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={() => insertFormat('```\n', '\n```', '// 代码')}
              className="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 transition-colors"
              title={t('codeBlock') || '代码块'}
            >
              <Code className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={() => insertFormat('- ', '', '列表项')}
              className="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 transition-colors"
              title={t('bulletList') || '无序列表'}
            >
              <List className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={() => insertFormat('1. ', '', '有序项')}
              className="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 transition-colors"
              title={t('orderedList') || '有序列表'}
            >
              <ListOrdered className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={() => insertFormat('- [ ] ', '', '待办事项')}
              className="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 transition-colors"
              title={t('taskList') || '任务清单'}
            >
              <CheckSquare className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={() => insertFormat('| 标题 1 | 标题 2 |\n| --- | --- |\n| 内容 1 | 内容 2 |\n')}
              className="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 transition-colors"
              title={t('table') || '表格'}
            >
              <Table className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={() => insertFormat('[', '](https://example.com)', '链接描述')}
              className="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 transition-colors"
              title={t('link') || '插入链接'}
            >
              <LinkIcon className="w-3.5 h-3.5" />
            </button>

            {/* 插入/上传图片按钮 */}
            <button
              type="button"
              onClick={handleImageButtonClick}
              disabled={isUploadingImage}
              className="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-indigo-600 dark:text-indigo-400 transition-colors relative"
              title={t('image') || '插入/上传图片 (支持本地上传、Ctrl+V 粘贴截图与拖拽)'}
            >
              {isUploadingImage ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <ImageIcon className="w-3.5 h-3.5" />
              )}
            </button>

            <button
              type="button"
              onClick={() => insertFormat('\n---\n')}
              className="p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-600 dark:text-gray-300 transition-colors"
              title={t('hr') || '分割线'}
            >
              <Minus className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        {/* Right: Stats, Dirty Indicator & Action Buttons */}
        <div className="flex items-center gap-2 text-gray-500 dark:text-gray-400 ml-auto">
          {/* Unsaved indicator */}
          {isDirty && (
            <div className="flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-amber-50 dark:bg-amber-950/40 text-amber-600 dark:text-amber-400 border border-amber-200 dark:border-amber-800 text-[11px] font-medium animate-pulse">
              <span className="w-1.5 h-1.5 rounded-full bg-amber-500" />
              <span>{t('unsavedChanges') || '未保存'}</span>
            </div>
          )}

          {/* Reset button if dirty */}
          {isDirty && (
            <button
              type="button"
              onClick={handleResetContent}
              className="p-1 rounded text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors"
              title={t('discard') || '放弃修改'}
            >
              <RotateCcw className="w-3.5 h-3.5" />
            </button>
          )}

          {/* Lines & Words Stats */}
          <div className="hidden lg:flex items-center gap-2 text-[11px] text-gray-400 dark:text-gray-500">
            <span className="flex items-center gap-0.5">
              <Hash className="w-3 h-3" />
              {stats.lines} {t('lines') || '行'}
            </span>
            <span>•</span>
            <span className="flex items-center gap-0.5">
              <AlignLeft className="w-3 h-3" />
              {stats.words} {t('words') || '字'}
            </span>
          </div>

          {/* Copy Button */}
          <button
            type="button"
            onClick={handleCopy}
            className="flex items-center gap-1 px-2 py-1 rounded border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-300 transition-colors text-xs"
            title="复制 Markdown 原文"
          >
            {copied ? (
              <>
                <Check className="w-3.5 h-3.5 text-emerald-500" />
                <span className="text-emerald-600 dark:text-emerald-400 text-xs">已复制</span>
              </>
            ) : (
              <>
                <Copy className="w-3.5 h-3.5" />
                <span className="hidden sm:inline text-xs">复制</span>
              </>
            )}
          </button>

          {/* Save Button */}
          {canSave && (
            <button
              type="button"
              onClick={handleSave}
              disabled={saving || !isDirty}
              className={clsx(
                'flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-medium shadow-xs transition-all',
                saveSuccess
                  ? 'bg-emerald-600 text-white'
                  : isDirty
                  ? 'bg-primary-600 hover:bg-primary-700 active:bg-primary-800 text-white'
                  : 'bg-gray-100 dark:bg-gray-800 text-gray-400 dark:text-gray-500 cursor-not-allowed border border-gray-200 dark:border-gray-700'
              )}
              title="保存 (Ctrl + S)"
            >
              {saving ? (
                <>
                  <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  <span>{t('saving') || '保存中...'}</span>
                </>
              ) : saveSuccess ? (
                <>
                  <Check className="w-3.5 h-3.5 text-white" />
                  <span>{t('saved') || '已保存'}</span>
                </>
              ) : (
                <>
                  <Save className="w-3.5 h-3.5" />
                  <span>{t('save') || '保存'}</span>
                </>
              )}
            </button>
          )}
        </div>
      </div>

      {/* Main Content Area */}
      <div className="relative flex-1 min-h-0 w-full overflow-hidden">
        {/* Rendered Mode */}
        {viewMode === 'rendered' && (
          <div
            ref={renderedContainerRef}
            className="h-full w-full overflow-y-auto px-6 py-8 md:px-12 lg:px-16 scroll-smooth bg-white dark:bg-gray-900"
          >
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
          <div ref={editorHostRef} className="h-full w-full overflow-hidden bg-white dark:bg-gray-900" />
        )}

        {/* Split Mode */}
        {viewMode === 'split' && (
          <div className="grid grid-cols-2 h-full w-full divide-x divide-gray-200 dark:divide-gray-800">
            <div ref={editorHostRef} className="h-full w-full overflow-hidden bg-white dark:bg-gray-900" />
            <div
              ref={splitRenderedContainerRef}
              className="h-full w-full overflow-y-auto px-6 py-6 scroll-smooth bg-white dark:bg-gray-900"
            >
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
