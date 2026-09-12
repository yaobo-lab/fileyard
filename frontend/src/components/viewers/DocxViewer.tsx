import React, { useState, useEffect, useRef, useCallback } from 'react';
import * as docx from 'docx-preview';
import {
  PanelLeft,
  MinusCircle,
  PlusCircle,
  MoreHorizontal,
  Loader2,
  Download,
  Moon,
  Sun,
  Copy,
  Check,
} from 'lucide-react';

interface DocxViewerProps {
  url: string;
  fileName: string;
  isDark?: boolean;
}

const ZOOM_OPTIONS = [10, 25, 50, 75, 100, 125, 150, 175, 200, 400];
const DEFAULT_ZOOM = 50;

export function DocxViewer({ url, fileName, isDark: initialIsDark = false }: DocxViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mainScrollRef = useRef<HTMLDivElement>(null);
  const sidebarScrollRef = useRef<HTMLDivElement>(null);
  const pageInputRef = useRef<HTMLInputElement>(null);

  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [zoom, setZoom] = useState<number>(DEFAULT_ZOOM);
  const [activePage, setActivePage] = useState<number>(1);
  const [totalPages, setTotalPages] = useState<number>(1);
  const [sidebarOpen, setSidebarOpen] = useState<boolean>(true);
  const [pageElements, setPageElements] = useState<HTMLElement[]>([]);
  const [isEditingPage, setIsEditingPage] = useState<boolean>(false);
  const [draftPage, setDraftPage] = useState<string>('1');

  // Menus & toggles
  const [isZoomMenuOpen, setIsZoomMenuOpen] = useState(false);
  const [isMoreMenuOpen, setIsMoreMenuOpen] = useState(false);
  const [isNightMode, setIsNightMode] = useState(initialIsDark);
  const [copied, setCopied] = useState(false);

  // Close menus on outside click
  const zoomMenuRef = useRef<HTMLDivElement>(null);
  const moreMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (zoomMenuRef.current && !zoomMenuRef.current.contains(e.target as Node)) {
        setIsZoomMenuOpen(false);
      }
      if (moreMenuRef.current && !moreMenuRef.current.contains(e.target as Node)) {
        setIsMoreMenuOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Clean wrapper styling helper to eliminate the ugly gray frame
  const applyCleanStyles = (root: HTMLElement) => {
    const wrappers = root.querySelectorAll<HTMLElement>('.docx-wrapper');
    wrappers.forEach((wrapper) => {
      wrapper.style.setProperty('background', 'transparent', 'important');
      wrapper.style.setProperty('padding', '0', 'important');
      wrapper.style.setProperty('margin', '0', 'important');
      wrapper.style.setProperty('box-shadow', 'none', 'important');
      wrapper.style.setProperty('border', 'none', 'important');
      wrapper.style.setProperty('display', 'flex', 'important');
      wrapper.style.setProperty('flex-direction', 'column', 'important');
      wrapper.style.setProperty('align-items', 'center', 'important');
    });

    const sections = root.querySelectorAll<HTMLElement>(
      '.docx-wrapper > section, section.docx-rendered-document, section.docx, section'
    );
    sections.forEach((section) => {
      section.style.setProperty('background', 'white', 'important');
      section.style.setProperty(
        'box-shadow',
        '0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px -1px rgba(0, 0, 0, 0.05)',
        'important'
      );
      section.style.setProperty('margin', '0 0 24px 0', 'important');
      section.style.setProperty('border', 'none', 'important');
      section.style.setProperty('outline', 'none', 'important');
    });
  };

  // Render docx
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setPageElements([]);
    setActivePage(1);

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
          // ignoreLastRenderedPageBreak: false ensures Word's authentic pagination (14 pages) is parsed
          await docx.renderAsync(blob, containerRef.current, undefined, {
            className: 'docx-rendered-document',
            inWrapper: true,
            ignoreWidth: false,
            ignoreHeight: false,
            breakPages: true,
            ignoreLastRenderedPageBreak: false,
            useBase64URL: true,
            renderHeaders: true,
            renderFooters: true,
            renderFootnotes: true,
            renderEndnotes: true,
            trimXmlDeclaration: true,
          });

          if (cancelled || !containerRef.current) return;

          // Strip any gray wrapper backgrounds and frames
          applyCleanStyles(containerRef.current);

          // Collect all rendered page sections
          const sections = Array.from(
            containerRef.current.querySelectorAll<HTMLElement>(
              '.docx-wrapper > section, section.docx-rendered-document, section.docx, section'
            )
          );

          if (sections.length > 0) {
            sections.forEach((sec, i) => {
              sec.setAttribute('data-docx-page', String(i + 1));
            });
            setPageElements(sections);
            setTotalPages(sections.length);
          } else {
            setPageElements([containerRef.current]);
            setTotalPages(1);
          }

          setLoading(false);
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

  // Sync active page with scroll
  const handleMainScroll = useCallback(() => {
    if (!mainScrollRef.current || pageElements.length === 0) return;

    const containerRect = mainScrollRef.current.getBoundingClientRect();
    const probeY = containerRect.top + 160;

    let closestPage = 1;
    let minDistance = Infinity;

    pageElements.forEach((section, idx) => {
      const rect = section.getBoundingClientRect();
      const distance = Math.abs(rect.top - probeY);
      if (distance < minDistance) {
        minDistance = distance;
        closestPage = idx + 1;
      }
    });

    setActivePage(closestPage);
  }, [pageElements]);

  // Jump to specific page
  const scrollToPage = useCallback(
    (pageNumber: number) => {
      const targetIndex = Math.max(1, Math.min(pageNumber, totalPages)) - 1;
      const targetElement = pageElements[targetIndex];

      if (targetElement && mainScrollRef.current) {
        setActivePage(targetIndex + 1);
        targetElement.scrollIntoView({ behavior: 'smooth', block: 'start' });
      }
    },
    [pageElements, totalPages]
  );

  // Zoom controls
  const handleZoomIn = () => {
    const next = ZOOM_OPTIONS.find((opt) => opt > zoom);
    if (next) setZoom(next);
    else if (zoom < 400) setZoom(Math.min(400, zoom + 25));
  };

  const handleZoomOut = () => {
    const prev = [...ZOOM_OPTIONS].reverse().find((opt) => opt < zoom);
    if (prev) setZoom(prev);
    else if (zoom > 10) setZoom(Math.max(10, zoom - 25));
  };

  const handleApplyDraftPage = () => {
    setIsEditingPage(false);
    const parsed = parseInt(draftPage, 10);
    if (!isNaN(parsed)) {
      scrollToPage(parsed);
    }
  };

  const handleCopyFileName = () => {
    navigator.clipboard.writeText(fileName);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
    setIsMoreMenuOpen(false);
  };

  const handleDownloadFile = () => {
    const a = document.createElement('a');
    a.href = url;
    a.download = fileName;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    setIsMoreMenuOpen(false);
  };

  return (
    <div
      className={`flex flex-col h-full w-full select-text ${
        isNightMode ? 'dark bg-gray-950 text-gray-100' : 'bg-white text-gray-800'
      }`}
    >
      {/* 1. Secondary Control Toolbar (1:1 with storageui) */}
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 px-3 z-10 select-none">
        {/* Left: Sidebar toggle + Page status */}
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={() => setSidebarOpen((prev) => !prev)}
            className={`p-1.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800 text-gray-600 dark:text-gray-300 transition-colors ${
              sidebarOpen ? 'bg-gray-100/80 dark:bg-gray-800' : ''
            }`}
            title={sidebarOpen ? '收起缩略图侧边栏' : '展开缩略图侧边栏'}
          >
            <PanelLeft className="w-4 h-4" />
          </button>

          <div className="flex items-center text-xs text-gray-600 dark:text-gray-300 font-normal">
            <span>第</span>
            {isEditingPage ? (
              <input
                ref={pageInputRef}
                type="text"
                value={draftPage}
                autoFocus
                onChange={(e) => setDraftPage(e.target.value)}
                onBlur={handleApplyDraftPage}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleApplyDraftPage();
                  if (e.key === 'Escape') setIsEditingPage(false);
                }}
                className="w-10 text-center mx-1 py-0.5 border border-primary-500 rounded bg-white dark:bg-gray-800 text-xs text-gray-900 dark:text-gray-100 outline-none"
              />
            ) : (
              <button
                type="button"
                onClick={() => {
                  setDraftPage(String(activePage));
                  setIsEditingPage(true);
                  setTimeout(() => pageInputRef.current?.select(), 50);
                }}
                className="mx-1 px-1.5 py-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800 font-normal text-gray-900 dark:text-gray-100 transition-colors"
                title="点击输入页码跳页"
              >
                {activePage}
              </button>
            )}
            <span>/ {totalPages} 页</span>
          </div>
        </div>

        {/* Right: Zoom controls + More options */}
        <div className="flex items-center gap-1.5">
          {/* Zoom Out Button */}
          <button
            type="button"
            onClick={handleZoomOut}
            disabled={zoom <= 10}
            className="p-1 text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white disabled:opacity-30 disabled:hover:text-gray-500 transition-colors"
            title="缩小"
          >
            <MinusCircle className="w-4 h-4" />
          </button>

          {/* Zoom Dropdown */}
          <div className="relative" ref={zoomMenuRef}>
            <button
              type="button"
              onClick={() => setIsZoomMenuOpen((prev) => !prev)}
              className="flex items-center justify-between gap-1 w-18 px-2 py-0.5 rounded border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 text-xs text-gray-700 dark:text-gray-200 hover:bg-gray-50 dark:hover:bg-gray-750 transition-colors shadow-2xs"
            >
              <span>{Math.round(zoom)}%</span>
              <span className="text-[10px] text-gray-400">▾</span>
            </button>

            {isZoomMenuOpen && (
              <div className="absolute right-0 top-full mt-1 w-24 py-1 bg-white dark:bg-gray-800 rounded-md shadow-lg border border-gray-200 dark:border-gray-700 z-50 text-xs max-h-60 overflow-y-auto">
                {ZOOM_OPTIONS.map((opt) => (
                  <button
                    key={opt}
                    type="button"
                    onClick={() => {
                      setZoom(opt);
                      setIsZoomMenuOpen(false);
                    }}
                    className={`w-full px-3 py-1.5 text-left flex items-center justify-between hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors ${
                      zoom === opt
                        ? 'font-semibold text-primary-600 dark:text-primary-400 bg-primary-50/40 dark:bg-primary-950/20'
                        : 'text-gray-700 dark:text-gray-200'
                    }`}
                  >
                    <span>{opt}%</span>
                    {zoom === opt && <Check className="w-3 h-3 text-primary-600 dark:text-primary-400" />}
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Zoom In Button */}
          <button
            type="button"
            onClick={handleZoomIn}
            disabled={zoom >= 400}
            className="p-1 text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white disabled:opacity-30 disabled:hover:text-gray-500 transition-colors"
            title="放大"
          >
            <PlusCircle className="w-4 h-4" />
          </button>

          {/* More Options Menu */}
          <div className="relative ml-1" ref={moreMenuRef}>
            <button
              type="button"
              onClick={() => setIsMoreMenuOpen((prev) => !prev)}
              className="p-1 text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white rounded hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
              title="更多选项"
            >
              <MoreHorizontal className="w-4 h-4" />
            </button>

            {isMoreMenuOpen && (
              <div className="absolute right-0 top-full mt-1 w-36 py-1 bg-white dark:bg-gray-800 rounded-md shadow-lg border border-gray-200 dark:border-gray-700 z-50 text-xs">
                <button
                  type="button"
                  onClick={() => {
                    setIsNightMode((prev) => !prev);
                    setIsMoreMenuOpen(false);
                  }}
                  className="w-full px-3 py-1.5 text-left flex items-center gap-2 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-200 transition-colors"
                >
                  {isNightMode ? <Sun className="w-3.5 h-3.5" /> : <Moon className="w-3.5 h-3.5" />}
                  <span>{isNightMode ? '明亮模式' : '夜间模式'}</span>
                </button>
                <button
                  type="button"
                  onClick={handleDownloadFile}
                  className="w-full px-3 py-1.5 text-left flex items-center gap-2 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-200 transition-colors"
                >
                  <Download className="w-3.5 h-3.5" />
                  <span>下载原文件</span>
                </button>
                <button
                  type="button"
                  onClick={handleCopyFileName}
                  className="w-full px-3 py-1.5 text-left flex items-center gap-2 hover:bg-gray-100 dark:hover:bg-gray-700 text-gray-700 dark:text-gray-200 transition-colors"
                >
                  {copied ? <Check className="w-3.5 h-3.5 text-emerald-500" /> : <Copy className="w-3.5 h-3.5" />}
                  <span>{copied ? '已复制' : '复制文件名'}</span>
                </button>
              </div>
            )}
          </div>
        </div>
      </div>

      {/* 2. Main Body: Sidebar + Document Canvas */}
      <div className="relative flex flex-1 min-h-0 min-w-0 overflow-hidden">
        {/* Left Thumbnail Sidebar */}
        {sidebarOpen && (
          <div
            ref={sidebarScrollRef}
            className="w-28 shrink-0 border-r border-gray-200 dark:border-gray-800 bg-[#f8f9fa] dark:bg-gray-900/90 overflow-y-auto p-2.5 flex flex-col gap-3 select-none"
          >
            {loading ? (
              <div className="flex flex-col items-center gap-2 py-8 text-gray-400">
                <Loader2 className="w-5 h-5 animate-spin text-primary-500" />
                <span className="text-[11px]">加载页码...</span>
              </div>
            ) : (
              Array.from({ length: totalPages }).map((_, index) => {
                const pageNumber = index + 1;
                const isSelected = pageNumber === activePage;
                const section = pageElements[index];

                return (
                  <div
                    key={pageNumber}
                    onClick={() => scrollToPage(pageNumber)}
                    className={`group cursor-pointer flex flex-col items-center gap-1.5 p-1.5 rounded-lg transition-all ${
                      isSelected
                        ? 'bg-gray-200/90 dark:bg-gray-800 ring-1 ring-gray-300 dark:ring-gray-700'
                        : 'hover:bg-gray-200/50 dark:hover:bg-gray-800/50'
                    }`}
                  >
                    {/* Page Thumbnail Paper Card */}
                    <div className="w-[80px] h-[112px] bg-white rounded-xs shadow-xs border border-gray-200/90 dark:border-gray-700 overflow-hidden relative flex flex-col justify-start">
                      {section ? (
                        <DocxPageMiniPreview section={section} />
                      ) : (
                        <div className="w-full h-full flex items-center justify-center bg-gray-50 text-[10px] text-gray-300">
                          {pageNumber}
                        </div>
                      )}
                    </div>

                    {/* Page Number Label */}
                    <span
                      className={`text-xs transition-colors ${
                        isSelected ? 'text-gray-900 dark:text-white font-medium' : 'text-gray-500 dark:text-gray-400'
                      }`}
                    >
                      {pageNumber}
                    </span>
                  </div>
                );
              })
            )}
          </div>
        )}

        {/* Center Document Scroll Viewport (Pure white paper on neutral light gray background) */}
        <div
          ref={mainScrollRef}
          onScroll={handleMainScroll}
          className={`flex-1 min-h-0 min-w-0 overflow-auto flex flex-col items-center p-6 ${
            isNightMode ? 'bg-[#0f172a]' : 'bg-[#f4f4f5]'
          }`}
        >
          {loading && (
            <div className="flex flex-col items-center justify-center my-auto gap-3 text-gray-500">
              <Loader2 className="w-8 h-8 animate-spin text-primary-600" />
              <p className="text-sm font-medium">正在解析 Word 文档排版与内容...</p>
            </div>
          )}

          {error && (
            <div className="flex flex-col items-center justify-center my-auto p-8 text-center max-w-md bg-white dark:bg-gray-800 rounded-xl shadow-xs border border-red-200 dark:border-red-900/30">
              <p className="text-sm font-medium text-red-500 mb-2">{error}</p>
              <p className="text-xs text-gray-500 mb-4">建议下载文件后使用本地 Word 软件查看</p>
              <button
                type="button"
                onClick={handleDownloadFile}
                className="inline-flex items-center gap-2 px-3 py-1.5 bg-primary-600 hover:bg-primary-700 text-white text-xs font-medium rounded-md shadow-xs transition-colors"
              >
                <Download className="w-3.5 h-3.5" />
                下载文件
              </button>
            </div>
          )}

          {/* Scaled Word Paper Flow */}
          <div
            className={`w-full flex flex-col items-center transition-transform duration-100 ease-out ${
              loading || error ? 'hidden' : 'block'
            }`}
            style={{
              zoom: zoom / 100,
            }}
          >
            <div
              ref={containerRef}
              className={`docx-preview-root ${isNightMode ? 'docx-night-render' : ''}`}
            />
          </div>
        </div>
      </div>

      {/* Global Style overrides to completely eradicate the gray frame and match 1:1 with storageui */}
      <style>{`
        .docx-preview-root .docx-wrapper,
        .docx-wrapper {
          background: transparent !important;
          padding: 0 !important;
          margin: 0 !important;
          border: none !important;
          box-shadow: none !important;
          display: flex !important;
          flex-direction: column !important;
          align-items: center !important;
        }
        .docx-preview-root .docx-wrapper > section.docx-rendered-document,
        .docx-preview-root .docx-wrapper > section.docx,
        .docx-preview-root .docx-wrapper > section,
        .docx-wrapper > section {
          margin-bottom: 24px !important;
          margin-left: 0 !important;
          margin-right: 0 !important;
          margin-top: 0 !important;
          border: none !important;
          outline: none !important;
          box-shadow: 0 1px 3px 0 rgba(0, 0, 0, 0.1), 0 1px 2px -1px rgba(0, 0, 0, 0.05) !important;
          background-color: white !important;
        }
        .docx-night-render .docx-wrapper > section.docx-rendered-document,
        .docx-night-render .docx-wrapper > section {
          filter: invert(0.9) hue-rotate(180deg);
        }
      `}</style>
    </div>
  );
}

/**
 * DocxPageMiniPreview renders an exact, unclipped 1:1 thumbnail replica of the target section
 */
function DocxPageMiniPreview({ section }: { section: HTMLElement }) {
  const mountRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!mountRef.current || !section) return;

    mountRef.current.innerHTML = '';
    const clone = section.cloneNode(true) as HTMLElement;

    // Reset styles on clone to ensure clean thumbnail paper look
    clone.style.margin = '0';
    clone.style.boxShadow = 'none';
    clone.style.border = 'none';
    clone.style.borderRadius = '0';
    clone.style.pointerEvents = 'none';
    clone.style.overflow = 'hidden';
    clone.style.background = 'white';

    mountRef.current.appendChild(clone);
  }, [section]);

  const targetWidth = section.offsetWidth || 794;
  const targetHeight = section.offsetHeight || 1123;
  const scale = 80 / targetWidth;

  return (
    <div
      ref={mountRef}
      style={{
        width: `${targetWidth}px`,
        height: `${targetHeight}px`,
        transform: `scale(${scale})`,
        transformOrigin: 'top left',
        pointerEvents: 'none',
        userSelect: 'none',
      }}
    />
  );
}
