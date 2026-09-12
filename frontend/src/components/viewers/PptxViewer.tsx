import React, { useState, useEffect, useCallback } from 'react';
import JSZip from 'jszip';
import { Loader2, ChevronLeft, ChevronRight, Presentation, Maximize2, Minimize2 } from 'lucide-react';
import clsx from 'clsx';

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
  const [isFullscreen, setIsFullscreen] = useState(false);

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
        if (!res.ok) throw new Error(`下载 PPT 演示文稿失败 (${res.status})`);
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

  // Keyboard navigation
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'ArrowLeft' || e.key === 'PageUp') {
        handlePrev();
      } else if (e.key === 'ArrowRight' || e.key === 'PageDown' || e.key === ' ') {
        handleNext();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handlePrev, handleNext]);

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center p-8 text-center">
        <p className="text-sm font-medium text-red-500 mb-2">{error}</p>
        <p className="text-xs text-gray-500">建议点击右上角下载按钮在 PowerPoint 中查看</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center gap-2">
        <Loader2 className="w-6 h-6 animate-spin text-primary-600" />
        <span className="text-sm text-gray-500">正在解析 PPT 幻灯片与图文内容...</span>
      </div>
    );
  }

  if (slides.length === 0) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-gray-500">
        该演示文稿中未包含任何有效幻灯片
      </div>
    );
  }

  const currentSlide = slides[currentSlideIndex];

  return (
    <div className={clsx(
      "h-full w-full flex flex-col select-text",
      isDark ? "bg-gray-950 text-gray-100" : "bg-gray-100 text-gray-900"
    )}>
      {/* Top Controller */}
      <div className={clsx(
        "flex items-center justify-between px-4 py-2 border-b text-xs flex-shrink-0",
        isDark ? "bg-gray-900 border-gray-800" : "bg-white border-gray-200 shadow-sm"
      )}>
        <div className="flex items-center gap-2 font-medium">
          <Presentation className="w-4 h-4 text-orange-500" />
          <span className="truncate max-w-xs">{fileName}</span>
        </div>

        <div className="flex items-center gap-2">
          <button
            onClick={handlePrev}
            disabled={currentSlideIndex === 0}
            className="p-1 rounded-md hover:bg-gray-200 dark:hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            title="上一张 (←)"
          >
            <ChevronLeft className="w-4 h-4" />
          </button>
          <span className="font-medium px-2 py-0.5 rounded bg-gray-200 dark:bg-gray-800">
            {currentSlideIndex + 1} / {slides.length}
          </span>
          <button
            onClick={handleNext}
            disabled={currentSlideIndex === slides.length - 1}
            className="p-1 rounded-md hover:bg-gray-200 dark:hover:bg-gray-800 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
            title="下一张 (→)"
          >
            <ChevronRight className="w-4 h-4" />
          </button>
        </div>

        <button
          onClick={() => setIsFullscreen(!isFullscreen)}
          className="p-1 rounded-md hover:bg-gray-200 dark:hover:bg-gray-800 transition-colors"
          title={isFullscreen ? "退出全屏" : "全屏放映"}
        >
          {isFullscreen ? <Minimize2 className="w-4 h-4" /> : <Maximize2 className="w-4 h-4" />}
        </button>
      </div>

      {/* Main Slide Presentation Stage */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Thumbnails List */}
        {!isFullscreen && (
          <div className={clsx(
            "w-48 overflow-y-auto p-3 flex flex-col gap-2.5 border-r flex-shrink-0 scrollbar-thin",
            isDark ? "bg-gray-900/50 border-gray-800" : "bg-gray-50 border-gray-200"
          )}>
            {slides.map((slide, idx) => (
              <button
                key={idx}
                onClick={() => setCurrentSlideIndex(idx)}
                className={clsx(
                  "flex flex-col text-left p-2 rounded-lg border text-xs transition-all relative overflow-hidden",
                  idx === currentSlideIndex
                    ? "border-primary-500 ring-2 ring-primary-500/20 bg-white dark:bg-gray-800 font-semibold shadow-sm"
                    : "border-gray-200 dark:border-gray-700 bg-white/70 dark:bg-gray-800/40 hover:bg-white dark:hover:bg-gray-800"
                )}
              >
                <div className="flex items-center justify-between mb-1 text-[10px] text-gray-400">
                  <span>第 {idx + 1} 页</span>
                  {slide.images.length > 0 && <span>🖼 {slide.images.length}</span>}
                </div>
                <div className="truncate font-medium text-gray-800 dark:text-gray-200">
                  {slide.title}
                </div>
              </button>
            ))}
          </div>
        )}

        {/* Current Slide Display Canvas */}
        <div className="flex-1 overflow-auto p-4 sm:p-8 flex items-center justify-center">
          <div className={clsx(
            "w-full max-w-4xl aspect-[16/9] rounded-xl shadow-2xl p-6 sm:p-10 flex flex-col justify-between border transition-all duration-200 relative overflow-hidden",
            isDark
              ? "bg-gray-900 border-gray-800 text-gray-100"
              : "bg-white border-gray-200 text-gray-900"
          )}>
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
                      className="max-h-60 max-w-full rounded-md object-contain shadow-sm hover:scale-105 transition-transform"
                    />
                  ))}
                </div>
              )}
            </div>

            {/* Slide Footer */}
            <div className="flex items-center justify-between pt-2 border-t border-gray-100 dark:border-gray-800 text-[10px] text-gray-400">
              <span>{fileName}</span>
              <span>第 {currentSlideIndex + 1} 页 / 共 {slides.length} 页</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
