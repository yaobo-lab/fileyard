import React, { useState } from 'react';
import { ZoomIn, ZoomOut, RotateCcw } from 'lucide-react';

interface DrawioViewerProps {
  url: string;
  fileName: string;
  isDark?: boolean;
}

export function DrawioViewer({ url, fileName, isDark = false }: DrawioViewerProps) {
  const [zoom, setZoom] = useState(1);
  const [error, setError] = useState(false);

  // draw.io diagrams can be displayed via standard diagrams.net viewer iframe or direct svg/image
  const isImageOrSvg = fileName.endsWith('.svg') || fileName.endsWith('.png');

  if (isImageOrSvg) {
    return (
      <div className="relative flex flex-col h-full w-full bg-slate-50 dark:bg-slate-900 overflow-hidden">
        <div className="flex-1 overflow-auto p-4 flex items-center justify-center">
          <img
            src={url}
            alt={fileName}
            onError={() => setError(true)}
            style={{ transform: `scale(${zoom})`, transformOrigin: 'center center' }}
            className="max-w-none transition-transform duration-100"
          />
        </div>
        <div className="flex items-center justify-center gap-2 p-2 border-t border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-800">
          <button
            onClick={() => setZoom((z) => Math.max(0.2, z - 0.2))}
            className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-700"
            title="缩小"
          >
            <ZoomOut className="w-4 h-4" />
          </button>
          <span className="text-xs text-gray-500 min-w-[48px] text-center">
            {Math.round(zoom * 100)}%
          </span>
          <button
            onClick={() => setZoom((z) => Math.min(4, z + 0.2))}
            className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-700"
            title="放大"
          >
            <ZoomIn className="w-4 h-4" />
          </button>
          <button
            onClick={() => setZoom(1)}
            className="p-1 rounded hover:bg-gray-100 dark:hover:bg-gray-700 ml-2"
            title="重置缩放"
          >
            <RotateCcw className="w-4 h-4" />
          </button>
        </div>
      </div>
    );
  }

  // Native draw.io XML can be viewed using the official diagrams.net viewer URL
  const viewerUrl = `https://viewer.diagrams.net/?highlight=0000ff&edit=_blank&layers=1&nav=1&title=${encodeURIComponent(
    fileName
  )}#U${encodeURIComponent(url)}`;

  return (
    <div className="h-full w-full bg-white dark:bg-gray-900">
      <iframe
        src={viewerUrl}
        className="h-full w-full border-0"
        title={fileName}
      />
    </div>
  );
}
