import React, { useState, useEffect } from 'react';
import {
  createFileTreeIconResolver,
  getBuiltInSpriteSheet,
} from '@pierre/trees';
import clsx from 'clsx';
import { Folder, Layers } from 'lucide-react';

// 3D/Gradient Folder SVG from storageui
export const FOLDER_GLYPH_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 50" width="64" height="50"><defs><linearGradient id="fs-folder-back" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color="#3dabf5"/><stop offset="1" stop-color="#1d84dd"/></linearGradient><linearGradient id="fs-folder-front" x1="0" x2="0" y1="0" y2="1"><stop offset="0" stop-color="#7accfb"/><stop offset="1" stop-color="#37a0ef"/></linearGradient></defs><path d="M5 10c0-3.31 2.69-6 6-6h10.9c1.6 0 3.13.7 4.18 1.9l1.5 1.73a3.5 3.5 0 0 0 2.64 1.22H54c2.76 0 5 2.24 5 5V40c0 3.87-3.13 7-7 7H12c-3.87 0-7-3.13-7-7V10Z" fill="url(#fs-folder-back)"/><path d="M5 15.5h54V40c0 3.87-3.13 7-7 7H12c-3.87 0-7-3.13-7-7V15.5Z" fill="url(#fs-folder-front)"/></svg>`;

export const FOLDER_GLYPH_DATA_URL = `data:image/svg+xml,${encodeURIComponent(FOLDER_GLYPH_SVG)}`;

export function FileSystemFolderGlyph({ className }: { className?: string }) {
  return (
    <img
      src={FOLDER_GLYPH_DATA_URL}
      alt="Folder"
      aria-hidden="true"
      draggable={false}
      className={clsx("select-none pointer-events-none drop-shadow-sm", className)}
    />
  );
}

// Built-in @pierre/trees icons sprite sheet
export const FILE_ICON_SPRITE_SHEET = getBuiltInSpriteSheet("complete");

const { resolveIcon: resolveFileIcon } = createFileTreeIconResolver({
  colored: true,
  set: "complete",
});

export const FILE_ICON_COLORS: Record<string, [light: string, dark: string]> = {
  astro: ["#a631be", "#d568ea"],
  babel: ["#d5a910", "#ffd452"],
  bash: ["#199f43", "#5ecc71"],
  biome: ["#1a85d4", "#69b1ff"],
  bootstrap: ["#693acf", "#9d6afb"],
  browserslist: ["#d5a910", "#ffd452"],
  bun: ["#594c5b", "#79697b"],
  c: ["#1a85d4", "#69b1ff"],
  claude: ["#d47628", "#ffa359"],
  cpp: ["#1a85d4", "#69b1ff"],
  css: ["#693acf", "#9d6afb"],
  database: ["#a631be", "#d568ea"],
  default: ["#84848a", "#adadb1"],
  docker: ["#1a85d4", "#69b1ff"],
  eslint: ["#693acf", "#9d6afb"],
  git: ["#ff8c5b", "#d5512f"],
  go: ["#1ca1c7", "#68cdf2"],
  graphql: ["#d32a61", "#ff678d"],
  html: ["#d47628", "#ffa359"],
  image: ["#d32a61", "#ff678d"],
  javascript: ["#d5a910", "#ffd452"],
  json: ["#d47628", "#ffa359"],
  markdown: ["#199f43", "#5ecc71"],
  mcp: ["#17a5af", "#64d1db"],
  npm: ["#d52c36", "#ff6762"],
  oxc: ["#1ca1c7", "#68cdf2"],
  postcss: ["#d52c36", "#ff6762"],
  prettier: ["#17a5af", "#64d1db"],
  python: ["#1a85d4", "#69b1ff"],
  react: ["#1ca1c7", "#68cdf2"],
  ruby: ["#d52c36", "#ff6762"],
  rust: ["#d47628", "#ffa359"],
  sass: ["#d32a61", "#ff678d"],
  svelte: ["#d52c36", "#ff6762"],
  svg: ["#d47628", "#ffa359"],
  svgo: ["#199f43", "#5ecc71"],
  swift: ["#d47628", "#ffa359"],
  table: ["#17a5af", "#64d1db"],
  tailwind: ["#1ca1c7", "#68cdf2"],
  terraform: ["#693acf", "#9d6afb"],
  text: ["#84848a", "#adadb1"],
  typescript: ["#1a85d4", "#69b1ff"],
  vite: ["#a631be", "#d568ea"],
  vscode: ["#1a85d4", "#69b1ff"],
  vue: ["#199f43", "#5ecc71"],
  wasm: ["#693acf", "#9d6afb"],
  webpack: ["#1a85d4", "#69b1ff"],
  yml: ["#d52c36", "#ff6762"],
  zig: ["#d47628", "#ffa359"],
  zip: ["#d47628", "#ffa359"],
};

export function fileIconColorVariables(mode: 0 | 1) {
  return Object.entries(FILE_ICON_COLORS)
    .map(([token, colors]) => `--fs-file-icon-${token}: ${colors[mode]};`)
    .join(" ");
}

export const FILE_ICON_COLOR_CSS = `
:root { ${fileIconColorVariables(0)} --fs-selected-color-scheme: dark; }
.dark { ${fileIconColorVariables(1)} --fs-selected-color-scheme: light; }
.dark [data-file-system-on-light] { ${fileIconColorVariables(0)} }
[data-file-system-on-primary] { ${fileIconColorVariables(1)} }
.dark [data-file-system-on-primary] { ${fileIconColorVariables(0)} }
`;

export function FileSystemIconSpriteSheet() {
  return (
    <>
      <span
        aria-hidden="true"
        className="hidden"
        dangerouslySetInnerHTML={{ __html: FILE_ICON_SPRITE_SHEET }}
      />
      <style>{FILE_ICON_COLOR_CSS}</style>
    </>
  );
}

export function FileTypeIcon({
  fileName,
  className,
}: {
  fileName: string;
  className?: string;
}) {
  const icon = resolveFileIcon("file-tree-icon-file", fileName);

  return (
    <svg
      aria-hidden="true"
      viewBox={icon.viewBox ?? "0 0 16 16"}
      className={clsx("shrink-0 text-muted-foreground", className)}
      style={
        icon.token
          ? {
              color: `var(--fs-file-icon-${icon.token}, currentColor)`,
            }
          : undefined
      }
    >
      <use href={`#${icon.name}`} />
    </svg>
  );
}

export function fileExtension(name: string) {
  const dotIndex = name.lastIndexOf('.');
  return dotIndex === -1 ? '' : name.slice(dotIndex + 1).toLowerCase();
}

export function isImageFileName(name: string) {
  const ext = fileExtension(name);
  return ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp', 'ico'].includes(ext);
}

// In-memory cache for thumbnail blob URLs to prevent repeatedly fetching
const thumbnailCache = new Map<string, string>();

export function FileGenericPaper({
  fileName,
  className,
}: {
  fileName: string;
  className?: string;
}) {
  const ext = fileExtension(fileName);

  return (
    <div
      data-file-system-on-light=""
      className={clsx(
        "relative flex w-12 h-15 shrink-0 flex-col items-center justify-center gap-1.5 rounded-[4px] border border-gray-200/90 bg-white dark:bg-neutral-100 shadow-[0_1px_2px_rgba(0,0,0,0.06)] select-none",
        className
      )}
      style={{ aspectRatio: "0.78" }}
    >
      <FileTypeIcon fileName={fileName} className="w-5 h-5 min-h-4 min-w-4" />
      {ext ? (
        <span className="text-[10px] font-semibold tracking-wider uppercase text-neutral-400 dark:text-neutral-500 leading-none">
          {ext.length > 5 ? ext.slice(0, 4) : ext}
        </span>
      ) : null}
    </div>
  );
}

export function FileThumbnailImage({
  fileId,
  fileName,
  companyId,
  className,
}: {
  fileId: string;
  fileName: string;
  companyId?: string;
  className?: string;
}) {
  const [blobUrl, setBlobUrl] = useState<string | null>(() => thumbnailCache.get(fileId) || null);
  const [hasError, setHasError] = useState(false);

  useEffect(() => {
    if (blobUrl || hasError || !companyId) return;

    let isMounted = true;
    const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');

    fetch(`/api/download/${companyId}/${fileId}`, {
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    })
      .then((res) => {
        if (!res.ok) throw new Error('Image thumbnail failed');
        return res.blob();
      })
      .then((blob) => {
        if (!isMounted) return;
        const url = URL.createObjectURL(blob);
        thumbnailCache.set(fileId, url);
        setBlobUrl(url);
      })
      .catch(() => {
        if (isMounted) setHasError(true);
      });

    return () => {
      isMounted = false;
    };
  }, [fileId, companyId, blobUrl, hasError]);

  if (hasError || !blobUrl) {
    return <FileGenericPaper fileName={fileName} className={className} />;
  }

  return (
    <div
      className={clsx(
        "relative flex w-12 h-15 shrink-0 items-center justify-center overflow-hidden rounded-[4px] border border-gray-200/90 bg-white dark:bg-neutral-100 shadow-[0_1px_2px_rgba(0,0,0,0.06)]",
        className
      )}
      style={{ aspectRatio: "0.78" }}
    >
      <img
        src={blobUrl}
        alt={fileName}
        loading="lazy"
        draggable={false}
        className="size-full object-cover select-none"
      />
    </div>
  );
}

export function FileGlyphVisual({
  file,
  companyId,
  className,
}: {
  file: {
    id: string;
    name: string;
    type?: string;
    color?: string;
  };
  companyId?: string;
  className?: string;
}) {
  if (file.type === 'folder') {
    return <FileSystemFolderGlyph className={clsx("h-13 w-auto", className)} />;
  }

  if (file.type === 'group') {
    return (
      <div className="relative flex items-center justify-center">
        <FileSystemFolderGlyph className={clsx("h-13 w-auto opacity-85", className)} />
        <Layers
          className="absolute inset-0 m-auto w-5 h-5 drop-shadow"
          style={{ color: file.color || '#3B82F6' }}
        />
      </div>
    );
  }

  // Check if image file
  if (file.type === 'image' || isImageFileName(file.name)) {
    return (
      <FileThumbnailImage
        fileId={file.id}
        fileName={file.name}
        companyId={companyId}
        className={className}
      />
    );
  }

  // Default file generic paper card
  return <FileGenericPaper fileName={file.name} className={className} />;
}
