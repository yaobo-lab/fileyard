import React, { useState, useEffect, useCallback, useRef } from 'react';
import JSZip from 'jszip';
import {
  Loader2,
  ChevronLeft,
  ChevronRight,
  PanelLeft,
  Maximize2,
  Minimize2,
  MoreHorizontal,
  Download,
  Copy,
  Check,
  Image as ImageIcon,
} from 'lucide-react';
import clsx from 'clsx';
import { copyToClipboard } from '@/lib/utils';

interface PptxViewerProps {
  url: string;
  fileName: string;
  isDark?: boolean;
}

interface SlideData {
  index: number;
  title: string;
  paragraphs: string[];
  images: string[];
}

export function PptxViewer({ url, fileName, isDark = false }: PptxViewerProps) {
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [slides, setSlides] = useState<SlideData[]>([]);
  const [currentSlideIndex, setCurrentSlideIndex] = useState(0);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [isEditingPage, setIsEditingPage] = useState(false);
  const [draftPage, setDraftPage] = useState('1');

  // Menus
  const [isMoreMenuOpen, setIsMoreMenuOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const moreMenuRef = useRef<HTMLDivElement>(null);
  const pageInputRef = useRef<HTMLInputElement>(null);
  const thumbnailContainerRef = useRef<HTMLDivElement>(null);

  // Close menu on outside click
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (moreMenuRef.current && !moreMenuRef.current.contains(e.target as Node)) {
        setIsMoreMenuOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Parse PPTX
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    setCurrentSlideIndex(0);

    const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
    fetch(url, {
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    })
      .then((res) => {
        if (!res.ok) throw new Error(`下载演示文稿失败 (${res.status})`);
        return res.arrayBuffer();
      })
      .then(async (buffer) => {
        if (cancelled) return;
        try {
          const zip = await JSZip.loadAsync(buffer);

          // Find all slide xml files
          const slideFileKeys = Object.keys(zip.files).filter((path) =>
            /^ppt\/slides\/slide\d+\.xml$/i.test(path)
          );

          if (slideFileKeys.length === 0) {
            throw new Error('未在演示文稿中找到幻灯片内容');
          }

          // Sort naturally by slide number: slide1.xml, slide2.xml, ...
          slideFileKeys.sort((a, b) => {
            const numA = parseInt(a.match(/slide(\d+)\.xml/i)?.[1] || '0', 10);
            const numB = parseInt(b.match(/slide(\d+)\.xml/i)?.[1] || '0', 10);
            return numA - numB;
          });

          // Extract all media blobs
          const mediaMap: Record<string, string> = {};
          const mediaKeys = Object.keys(zip.files).filter((path) => /^ppt\/media\//i.test(path));
          for (const mediaKey of mediaKeys) {
            const mediaFile = zip.files[mediaKey];
            if (mediaFile && !mediaFile.dir) {
              const blob = await mediaFile.async('blob');
              mediaMap[mediaKey] = URL.createObjectURL(blob);
            }
          }

          const parsedSlides: SlideData[] = [];

          for (let i = 0; i < slideFileKeys.length; i++) {
            const slideKey = slideFileKeys[i];
            const slideXmlStr = await zip.files[slideKey].async('string');
            const parser = new DOMParser();
            const xmlDoc = parser.parseFromString(slideXmlStr, 'application/xml');

            // Find slide relationships to resolve images
            const relKey = slideKey.replace('ppt/slides/', 'ppt/slides/_rels/') + '.rels';
            const slideImages: string[] = [];

            if (zip.files[relKey]) {
              const relXmlStr = await zip.files[relKey].async('string');
              const relDoc = parser.parseFromString(relXmlStr, 'application/xml');
              const relNodes = relDoc.querySelectorAll('Relationship');
              relNodes.forEach((rel) => {
                const target = rel.getAttribute('Target') || '';
                if (/media\//i.test(target)) {
                  const resolvedPath = target.startsWith('..')
                    ? 'ppt/' + target.replace('../', '')
                    : 'ppt/slides/' + target;
                  if (mediaMap[resolvedPath]) {
                    slideImages.push(mediaMap[resolvedPath]);
                  }
                }
              });
            }

            // Extract paragraphs (<a:p>)
            const pNodes = xmlDoc.querySelectorAll('p');
            const paragraphs: string[] = [];
            pNodes.forEach((p) => {
              const tNodes = p.querySelectorAll('t');
              let pText = '';
              tNodes.forEach((t) => {
                pText += t.textContent || '';
              });
              pText = pText.trim();
              if (pText) {
                paragraphs.push(pText);
              }
            });

            const title = paragraphs[0] || `幻灯片 ${i + 1}`;
            const bodyParagraphs = paragraphs.length > 1 ? paragraphs.slice(1) : paragraphs;

            parsedSlides.push({
              index: i,
              title,
              paragraphs: bodyParagraphs,
              images: slideImages,
            });
          }

          if (!cancelled) {
            setSlides(parsedSlides);
            setLoading(false);
          }
        } catch (zipErr: any) {
          if (!cancelled) {
            setError(zipErr.message || 'PPTX 文档解析失败，请尝试下载后查看');
            setLoading(false);
          }
        }
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err.message || 'PPTX 下载失败');
          setLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [url, fileName]);

  const handlePrev = useCallback(() => {
    setCurrentSlideIndex((prev) => Math.max(0, prev - 1));
  }, []);

  const handleNext = useCallback(() => {
    setCurrentSlideIndex((prev) => Math.min(slides.length - 1, prev + 1));
  }, [slides.length]);

  // Jump to specific page
  const scrollToSlide = useCallback(
    (index: number) => {
      const validIndex = Math.max(0, Math.min(index, slides.length - 1));
      setCurrentSlideIndex(validIndex);
    },
    [slides.length]
  );

  const handleApplyDraftPage = () => {
    setIsEditingPage(false);
    const parsed = parseInt(draftPage, 10);
    if (!isNaN(parsed) && parsed >= 1 && parsed <= slides.length) {
      scrollToSlide(parsed - 1);
    }
  };

  // Keep active thumbnail in view
  useEffect(() => {
    if (!thumbnailContainerRef.current) return;
    const activeItem = thumbnailContainerRef.current.querySelector(
      `[data-slide-index="${currentSlideIndex}"]`
    ) as HTMLElement;
    if (activeItem) {
      activeItem.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }
  }, [currentSlideIndex]);

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (isEditingPage) return;
      if (e.key === 'ArrowLeft' || e.key === 'PageUp') {
        handlePrev();
      } else if (e.key === 'ArrowRight' || e.key === 'PageDown' || e.key === ' ') {
        handleNext();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handlePrev, handleNext, isEditingPage]);

  const handleCopyFileName = async () => {
    const ok = await copyToClipboard(fileName);
    if (ok) {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
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

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center p-8 text-center my-auto">
        <p className="text-sm font-medium text-red-500 mb-2">{error}</p>
        <p className="text-xs text-gray-500 mb-4">建议下载文件后使用本地 PowerPoint 软件查看</p>
        <button
          type="button"
          onClick={handleDownloadFile}
          className="inline-flex items-center gap-2 px-3 py-1.5 bg-primary-600 hover:bg-primary-700 text-white text-xs font-medium rounded-md shadow-xs transition-colors"
        >
          <Download className="w-3.5 h-3.5" />
          下载演示文稿
        </button>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center text-gray-500 my-auto">
        <Loader2 className="w-8 h-8 animate-spin text-primary-600" />
        <p className="text-sm font-medium">正在解析幻灯片内容与高清素材...</p>
      </div>
    );
  }

  const currentSlide = slides[currentSlideIndex] || slides[0];

  return (
    <div
      className={clsx(
        'flex flex-col h-full w-full select-text overflow-hidden',
        isDark ? 'dark bg-gray-950 text-gray-100' : 'bg-white text-gray-800'
      )}
    >
      {/* 1. Secondary Control Toolbar (1:1 with storageui design system) */}
      <div className="flex h-11 shrink-0 items-center justify-between border-b border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 px-3 z-10 select-none">
        {/* Left: Sidebar toggle + Page number control */}
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
                className="w-12 text-center mx-1 py-0.5 border border-primary-500 rounded bg-white dark:bg-gray-800 text-xs text-gray-900 dark:text-gray-100 outline-none"
              />
            ) : (
              <button
                type="button"
                onClick={() => {
                  setDraftPage(String(currentSlideIndex + 1));
                  setIsEditingPage(true);
                  setTimeout(() => pageInputRef.current?.select(), 50);
                }}
                className="mx-1 px-1.5 py-0.5 rounded hover:bg-gray-100 dark:hover:bg-gray-800 font-normal text-gray-900 dark:text-gray-100 transition-colors"
                title="点击输入页码跳页"
              >
                {currentSlideIndex + 1}
              </button>
            )}
            <span>/ {slides.length} 页</span>
          </div>
        </div>

        {/* Right: Slide Controls + Fullscreen + More options */}
        <div className="flex items-center gap-1.5">
          {/* Previous Slide Button */}
          <button
            type="button"
            onClick={handlePrev}
            disabled={currentSlideIndex === 0}
            className="p-1 text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white disabled:opacity-30 disabled:hover:text-gray-500 transition-colors"
            title="上一张 (←)"
          >
            <ChevronLeft className="w-4 h-4" />
          </button>

          <span className="text-xs text-gray-500 dark:text-gray-400 px-1 font-mono">
            {currentSlideIndex + 1} / {slides.length}
          </span>

          {/* Next Slide Button */}
          <button
            type="button"
            onClick={handleNext}
            disabled={currentSlideIndex === slides.length - 1}
            className="p-1 text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white disabled:opacity-30 disabled:hover:text-gray-500 transition-colors"
            title="下一张 (→)"
          >
            <ChevronRight className="w-4 h-4" />
          </button>

          {/* Fullscreen / Theater Toggle */}
          <button
            type="button"
            onClick={() => setIsFullscreen(!isFullscreen)}
            className="p-1 text-gray-500 hover:text-gray-900 dark:text-gray-400 dark:hover:text-white rounded hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors ml-1"
            title={isFullscreen ? '还原窗口' : '全屏放映'}
          >
            {isFullscreen ? <Minimize2 className="w-4 h-4" /> : <Maximize2 className="w-4 h-4" />}
          </button>

          {/* More Options Menu */}
          <div className="relative ml-0.5" ref={moreMenuRef}>
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

      {/* 2. Main Body: Thumbnail Sidebar + Presentation Stage */}
      <div className="relative flex flex-1 min-h-0 min-w-0 overflow-hidden">
        {/* Left 16:9 Thumbnail Sidebar */}
        {sidebarOpen && !isFullscreen && (
          <div
            ref={thumbnailContainerRef}
            className="w-38 shrink-0 border-r border-gray-200 dark:border-gray-800 bg-[#f8f9fa] dark:bg-gray-900/90 overflow-y-auto p-2.5 flex flex-col gap-3 select-none"
          >
            {slides.map((slide, idx) => {
              const isSelected = idx === currentSlideIndex;
              return (
                <div
                  key={idx}
                  data-slide-index={idx}
                  onClick={() => setCurrentSlideIndex(idx)}
                  className={`group cursor-pointer flex flex-col items-center gap-1.5 p-1.5 rounded-lg transition-all ${
                    isSelected
                      ? 'bg-gray-200/90 dark:bg-gray-800 ring-1 ring-gray-300 dark:ring-gray-700'
                      : 'hover:bg-gray-200/50 dark:hover:bg-gray-800/50'
                  }`}
                >
                  {/* 16:9 Slide Thumbnail Card */}
                  <div className="w-[124px] h-[70px] bg-white dark:bg-gray-850 rounded-xs shadow-xs border border-gray-200/90 dark:border-gray-700 overflow-hidden relative flex flex-col justify-start">
                    <SlideMiniThumbnail slide={slide} />
                  </div>

                  {/* Slide Page Number Label */}
                  <span
                    className={`text-xs transition-colors ${
                      isSelected ? 'text-gray-900 dark:text-white font-medium' : 'text-gray-500 dark:text-gray-400'
                    }`}
                  >
                    {idx + 1}
                  </span>
                </div>
              );
            })}
          </div>
        )}

        {/* Center Presentation Stage */}
        <div
          className={`flex-1 min-h-0 min-w-0 overflow-auto flex items-center justify-center p-4 sm:p-8 relative ${
            isDark ? 'bg-[#0f172a]' : 'bg-[#f4f4f5]'
          }`}
        >
          {/* Main 16:9 Canvas */}
          <div
            className={clsx(
              'w-full max-w-4xl aspect-[16/9] rounded-xl shadow-xl p-6 sm:p-10 flex flex-col justify-between border transition-all duration-200 relative overflow-hidden',
              isDark ? 'bg-gray-900 border-gray-800 text-gray-100' : 'bg-white border-gray-200/80 text-gray-900'
            )}
          >
            {/* Slide Header / Title */}
            <div className="mb-4">
              <h2 className="text-xl sm:text-2xl font-bold tracking-tight text-primary-600 dark:text-primary-400">
                {currentSlide.title}
              </h2>
              <div className="h-0.5 w-16 bg-primary-500 mt-2 rounded-full" />
            </div>

            {/* Slide Content: Paragraphs and Images */}
            <div className="flex-1 flex flex-col md:flex-row gap-6 my-2 overflow-auto">
              {/* Text Area */}
              <div className="flex-1 flex flex-col gap-2.5 overflow-y-auto">
                {currentSlide.paragraphs.map((p, pIdx) => (
                  <p key={pIdx} className="text-sm sm:text-base leading-relaxed text-gray-700 dark:text-gray-300">
                    {p}
                  </p>
                ))}
              </div>

              {/* Images Grid */}
              {currentSlide.images.length > 0 && (
                <div className="md:w-1/2 flex flex-wrap gap-2 items-center justify-center p-2 rounded-lg bg-gray-50 dark:bg-gray-800/50 border border-gray-100 dark:border-gray-800">
                  {currentSlide.images.map((imgUrl, imgIdx) => (
                    <img
                      key={imgIdx}
                      src={imgUrl}
                      alt={`Slide Image ${imgIdx + 1}`}
                      className="max-h-60 max-w-full rounded-md object-contain shadow-xs hover:scale-105 transition-transform"
                    />
                  ))}
                </div>
              )}
            </div>

            {/* Slide Footer */}
            <div className="flex items-center justify-between pt-2 border-t border-gray-100 dark:border-gray-800 text-[10px] text-gray-400">
              <span className="truncate max-w-xs">{fileName}</span>
              <span>
                第 {currentSlideIndex + 1} 页 / 共 {slides.length} 页
              </span>
            </div>
          </div>

          {/* Floating Previous & Next Navigation Buttons on canvas edges */}
          <button
            type="button"
            onClick={handlePrev}
            disabled={currentSlideIndex === 0}
            className="absolute left-3 top-1/2 -translate-y-1/2 p-2 rounded-full bg-white/80 dark:bg-gray-800/80 shadow-md hover:bg-white dark:hover:bg-gray-800 text-gray-700 dark:text-gray-200 disabled:opacity-0 pointer-events-auto transition-all"
            title="上一张 (←)"
          >
            <ChevronLeft className="w-5 h-5" />
          </button>

          <button
            type="button"
            onClick={handleNext}
            disabled={currentSlideIndex === slides.length - 1}
            className="absolute right-3 top-1/2 -translate-y-1/2 p-2 rounded-full bg-white/80 dark:bg-gray-800/80 shadow-md hover:bg-white dark:hover:bg-gray-800 text-gray-700 dark:text-gray-200 disabled:opacity-0 pointer-events-auto transition-all"
            title="下一张 (→)"
          >
            <ChevronRight className="w-5 h-5" />
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * SlideMiniThumbnail renders an authentic 16:9 thumbnail preview
 * displaying the slide's key graphics or text layout.
 */
function SlideMiniThumbnail({ slide }: { slide: SlideData }) {
  const hasImage = slide.images && slide.images.length > 0;

  if (hasImage) {
    return (
      <div className="w-full h-full relative bg-gray-50 dark:bg-gray-800 flex items-center justify-center overflow-hidden">
        {/* Main image preview */}
        <img
          src={slide.images[0]}
          alt={slide.title}
          className="w-full h-full object-cover"
        />
        {/* Bottom subtle title bar */}
        <div className="absolute inset-x-0 bottom-0 bg-black/60 backdrop-blur-2xs px-1 py-0.5">
          <p className="text-[8px] text-white truncate leading-tight">{slide.title}</p>
        </div>
      </div>
    );
  }

  // Pure text slide template thumbnail
  return (
    <div className="w-full h-full p-1.5 flex flex-col justify-between bg-white dark:bg-gray-800 select-none">
      {/* Title preview */}
      <div>
        <div className="h-1.5 w-12 bg-primary-500/80 rounded-full mb-1" />
        <p className="text-[8px] font-semibold text-gray-800 dark:text-gray-200 line-clamp-1 leading-none">
          {slide.title}
        </p>
      </div>

      {/* Mock paragraph lines */}
      <div className="flex flex-col gap-0.5 my-auto">
        <div className="h-1 w-full bg-gray-200 dark:bg-gray-700 rounded-xs" />
        <div className="h-1 w-4/5 bg-gray-200 dark:bg-gray-700 rounded-xs" />
        <div className="h-1 w-3/5 bg-gray-200 dark:bg-gray-700 rounded-xs" />
      </div>

      {/* Mini footer */}
      <div className="flex items-center justify-between">
        <div className="h-0.5 w-6 bg-gray-300 dark:bg-gray-600 rounded-full" />
        <div className="h-0.5 w-2 bg-gray-300 dark:bg-gray-600 rounded-full" />
      </div>
    </div>
  );
}
