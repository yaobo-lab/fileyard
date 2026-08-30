// Shortcut Preset Definitions
// Each preset defines a complete set of keyboard shortcuts for different user preferences

export type ShortcutPresetId = 'default' | 'vim' | 'minimal';

export interface ShortcutBinding {
  keys: string[];
  isSequence?: boolean;
}

export interface ShortcutDefinition {
  id: string;
  description: string;
  category: 'navigation' | 'files' | 'ui' | 'selection';
  binding: ShortcutBinding;
}

export interface ShortcutPreset {
  id: ShortcutPresetId;
  name: string;
  description: string;
  shortcuts: Record<string, ShortcutBinding>;
}

// All available shortcut action IDs
// Note: Avoid browser-reserved shortcuts like ⌘+K, ⌘+D, ⌘+N, ⌘+P, ⌘+U on Mac Chrome
export const SHORTCUT_ACTIONS = {
  // Navigation
  'nav.dashboard': { description: 'Go to Dashboard', category: 'navigation' as const },
  'nav.files': { description: 'Go to Files', category: 'navigation' as const },
  'nav.users': { description: 'Go to Users', category: 'navigation' as const },
  'nav.companies': { description: 'Go to Companies', category: 'navigation' as const },
  'nav.settings': { description: 'Go to Settings', category: 'navigation' as const },
  'nav.profile': { description: 'Go to Profile', category: 'navigation' as const },
  'nav.notifications': { description: 'Go to Notifications', category: 'navigation' as const },
  
  // UI Controls
  'ui.search': { description: 'Focus search', category: 'ui' as const },
  'ui.theme': { description: 'Toggle dark/light theme', category: 'ui' as const },
  'ui.help': { description: 'Show keyboard shortcuts', category: 'ui' as const },
  'ui.close': { description: 'Close modal or clear selection', category: 'ui' as const },
  
  // File Operations
  'file.upload': { description: 'Upload files', category: 'files' as const },
  'file.newFolder': { description: 'Create new folder', category: 'files' as const },
  'file.rename': { description: 'Rename selected file', category: 'files' as const },
  'file.move': { description: 'Move selected files', category: 'files' as const },
  'file.delete': { description: 'Delete selected files', category: 'files' as const },
  'file.open': { description: 'Open file or enter folder', category: 'files' as const },
  'file.download': { description: 'Download selected file', category: 'files' as const },
  'file.preview': { description: 'Preview file', category: 'files' as const },
  
  // Selection & Navigation
  'select.toggle': { description: 'Toggle selection on focused item', category: 'selection' as const },
  'select.all': { description: 'Select all files', category: 'selection' as const },
  'select.up': { description: 'Move focus up', category: 'selection' as const },
  'select.down': { description: 'Move focus down', category: 'selection' as const },
  'select.left': { description: 'Move focus left / Navigate out', category: 'selection' as const },
  'select.right': { description: 'Move focus right / Navigate into', category: 'selection' as const },
} as const;

export type ShortcutActionId = keyof typeof SHORTCUT_ACTIONS;

// Default Preset - Mnemonic, discoverable shortcuts
export const DEFAULT_PRESET: ShortcutPreset = {
  id: 'default',
  name: 'Default',
  description: 'Mnemonic shortcuts designed for discoverability (G+D, U for upload)',
  shortcuts: {
    // Navigation - G + key sequences
    'nav.dashboard': { keys: ['g', 'd'], isSequence: true },
    'nav.files': { keys: ['g', 'f'], isSequence: true },
    'nav.users': { keys: ['g', 'u'], isSequence: true },
    'nav.companies': { keys: ['g', 'c'], isSequence: true },
    'nav.settings': { keys: ['g', 's'], isSequence: true },
    'nav.profile': { keys: ['g', 'p'], isSequence: true },
    'nav.notifications': { keys: ['g', 'n'], isSequence: true },
    
    // UI Controls (avoiding browser-reserved shortcuts)
    'ui.search': { keys: ['/'] },
    'ui.theme': { keys: ['t'] },
    'ui.help': { keys: ['?'] },
    'ui.close': { keys: ['escape'] },
    
    // File Operations - Mnemonic (avoiding ⌘+D which is bookmark in Chrome)
    'file.upload': { keys: ['u'] },
    'file.newFolder': { keys: ['n'] },
    'file.rename': { keys: ['r'] },
    'file.move': { keys: ['m'] },
    'file.delete': { keys: ['delete'] },
    'file.open': { keys: ['enter'] },
    'file.download': { keys: ['d'] },
    'file.preview': { keys: ['p'] },
    
    // Selection - Arrow keys
    'select.toggle': { keys: ['space'] },
    'select.all': { keys: ['a'] },
    'select.up': { keys: ['arrowup'] },
    'select.down': { keys: ['arrowdown'] },
    'select.left': { keys: ['arrowleft'] },
    'select.right': { keys: ['arrowright'] },
  },
};

// Vim Preset - Vim-style navigation and operations
export const VIM_PRESET: ShortcutPreset = {
  id: 'vim',
  name: 'Vim',
  description: 'Vim-style navigation with j/k/h/l and modal operations',
  shortcuts: {
    // Navigation - G + key sequences (like vim goto)
    'nav.dashboard': { keys: ['g', 'd'], isSequence: true },
    'nav.files': { keys: ['g', 'f'], isSequence: true },
    'nav.users': { keys: ['g', 'u'], isSequence: true },
    'nav.companies': { keys: ['g', 'c'], isSequence: true },
    'nav.settings': { keys: ['g', 's'], isSequence: true },
    'nav.profile': { keys: ['g', 'p'], isSequence: true },
    'nav.notifications': { keys: ['g', 'n'], isSequence: true },
    
    // UI Controls
    'ui.search': { keys: ['/'] },
    'ui.theme': { keys: ['g', 't'], isSequence: true },
    'ui.help': { keys: ['?'] },
    'ui.close': { keys: ['escape'] },
    
    // File Operations - Vim style
    'file.upload': { keys: ['o'] },        // "open" (upload to open new files)
    'file.newFolder': { keys: ['O'] },     // "Open" above (create folder)
    'file.rename': { keys: ['c', 'w'], isSequence: true }, // "change word"
    'file.move': { keys: ['m'] },          // move
    'file.delete': { keys: ['d', 'd'], isSequence: true }, // delete line
    'file.open': { keys: ['enter'] },
    'file.download': { keys: ['y', 'y'], isSequence: true }, // yank (copy/download)
    'file.preview': { keys: ['v'] },       // view
    
    // Selection - Vim navigation
    'select.toggle': { keys: ['x'] },      // toggle mark
    'select.all': { keys: ['V'] },         // Visual line mode selects all
    'select.up': { keys: ['k'] },
    'select.down': { keys: ['j'] },
    'select.left': { keys: ['h'] },
    'select.right': { keys: ['l'] },
  },
};

// Minimal Preset - Simple, reduced shortcuts (arrow keys + essential operations only)
export const MINIMAL_PRESET: ShortcutPreset = {
  id: 'minimal',
  name: 'Minimal',
  description: 'Arrow keys and essential operations only - fewer shortcuts to remember',
  shortcuts: {
    // Navigation - G + key sequences (simple and non-conflicting)
    'nav.dashboard': { keys: ['g', 'd'], isSequence: true },
    'nav.files': { keys: ['g', 'f'], isSequence: true },
    'nav.users': { keys: ['g', 'u'], isSequence: true },
    'nav.companies': { keys: ['g', 'c'], isSequence: true },
    'nav.settings': { keys: ['g', 's'], isSequence: true },
    'nav.profile': { keys: ['g', 'p'], isSequence: true },
    'nav.notifications': { keys: ['g', 'n'], isSequence: true },
    
    // UI Controls - Minimal
    'ui.search': { keys: ['/'] },
    'ui.theme': { keys: ['t'] },
    'ui.help': { keys: ['?'] },
    'ui.close': { keys: ['escape'] },
    
    // File Operations - Only essential ones
    'file.upload': { keys: ['u'] },
    'file.newFolder': { keys: ['n'] },
    'file.rename': { keys: ['f2'] },
    'file.move': { keys: ['m'] },
    'file.delete': { keys: ['delete'] },
    'file.open': { keys: ['enter'] },
    'file.download': { keys: ['d'] },
    'file.preview': { keys: ['p'] },
    
    // Selection - Arrow keys only
    'select.toggle': { keys: ['space'] },
    'select.all': { keys: ['a'] },
    'select.up': { keys: ['arrowup'] },
    'select.down': { keys: ['arrowdown'] },
    'select.left': { keys: ['arrowleft'] },
    'select.right': { keys: ['arrowright'] },
  },
};

// All presets
export const PRESETS: Record<ShortcutPresetId, ShortcutPreset> = {
  default: DEFAULT_PRESET,
  vim: VIM_PRESET,
  minimal: MINIMAL_PRESET,
};

// Get a preset by ID
export function getPreset(id: ShortcutPresetId): ShortcutPreset {
  return PRESETS[id] || DEFAULT_PRESET;
}

// Merge custom overrides with a preset
export function mergeWithCustomBindings(
  preset: ShortcutPreset,
  customBindings: Record<string, ShortcutBinding>
): Record<string, ShortcutBinding> {
  return {
    ...preset.shortcuts,
    ...customBindings,
  };
}

// Convert 'mod' to actual platform modifier
export function resolveMod(keys: string[]): string[] {
  const modKey = navigator.platform.toLowerCase().includes('mac') ? 'meta' : 'ctrl';
  return keys.map(k => k === 'mod' ? modKey : k);
}

// Get all shortcuts with resolved bindings and descriptions
export function getResolvedShortcuts(
  bindings: Record<string, ShortcutBinding>
): ShortcutDefinition[] {
  const results: ShortcutDefinition[] = [];
  
  for (const [actionId, binding] of Object.entries(bindings)) {
    const action = SHORTCUT_ACTIONS[actionId as ShortcutActionId];
    if (action) {
      results.push({
        id: actionId,
        description: action.description,
        category: action.category,
        binding: {
          keys: resolveMod(binding.keys),
          isSequence: binding.isSequence,
        },
      });
    }
  }
  
  return results;
}

