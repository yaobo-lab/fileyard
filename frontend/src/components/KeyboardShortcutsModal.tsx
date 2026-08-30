import { useEffect, useRef, useState } from 'react';
import { X, Keyboard, ChevronDown, Check } from 'lucide-react';
import { useKeyboardShortcutsContext, formatShortcut } from '../context/KeyboardShortcutsContext';
import { getModifierKey } from '../hooks/useKeyboardShortcuts';
import { ShortcutPresetId } from '../hooks/shortcutPresets';

export function KeyboardShortcutsModal() {
  const { 
    isHelpOpen, 
    closeHelp, 
    categories,
    currentPresetId,
    presets,
    setPreset,
  } = useKeyboardShortcutsContext();
  const modalRef = useRef<HTMLDivElement>(null);
  const [isPresetDropdownOpen, setIsPresetDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const isMac = getModifierKey() === 'meta';

  // Close on escape
  useEffect(() => {
    if (!isHelpOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (isPresetDropdownOpen) {
          setIsPresetDropdownOpen(false);
        } else {
          closeHelp();
        }
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isHelpOpen, closeHelp, isPresetDropdownOpen]);

  // Close on click outside
  useEffect(() => {
    if (!isHelpOpen) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current?.contains(e.target as Node)) {
        return;
      }
      if (modalRef.current && !modalRef.current.contains(e.target as Node)) {
        closeHelp();
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [isHelpOpen, closeHelp]);

  // Close dropdown when clicking outside
  useEffect(() => {
    if (!isPresetDropdownOpen) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setIsPresetDropdownOpen(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [isPresetDropdownOpen]);

  const handlePresetChange = (presetId: ShortcutPresetId) => {
    setPreset(presetId);
    setIsPresetDropdownOpen(false);
  };

  if (!isHelpOpen) return null;

  const currentPreset = presets[currentPresetId];

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 backdrop-blur-sm">
      <div
        ref={modalRef}
        className="bg-white dark:bg-gray-800 rounded-xl shadow-2xl max-w-2xl w-full mx-4 max-h-[80vh] overflow-hidden flex flex-col animate-in fade-in zoom-in-95 duration-200"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-lg">
              <Keyboard className="w-5 h-5 text-primary-600 dark:text-primary-400" />
            </div>
            <div>
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                Keyboard Shortcuts
              </h2>
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {isMac ? 'Using ⌘ for modifier key' : 'Using Ctrl for modifier key'}
              </p>
            </div>
          </div>
          <button
            onClick={closeHelp}
            className="p-2 rounded-lg text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Preset Selector */}
        <div className="px-6 py-3 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/30">
          <div className="flex items-center gap-3">
            <span className="text-sm text-gray-600 dark:text-gray-400">Preset:</span>
            <div className="relative" ref={dropdownRef}>
              <button
                onClick={() => setIsPresetDropdownOpen(!isPresetDropdownOpen)}
                className="flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-gray-700 dark:text-gray-200 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-600 transition-colors"
              >
                {currentPreset.name}
                <ChevronDown className="w-4 h-4" />
              </button>
              
              {isPresetDropdownOpen && (
                <div className="absolute left-0 top-full mt-1 w-64 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg z-10 overflow-hidden">
                  {Object.values(presets).map((preset) => (
                    <button
                      key={preset.id}
                      onClick={() => handlePresetChange(preset.id)}
                      className="w-full flex items-start gap-3 px-4 py-3 text-left hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                    >
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-medium text-gray-900 dark:text-white">
                            {preset.name}
                          </span>
                          {currentPresetId === preset.id && (
                            <Check className="w-4 h-4 text-primary-500" />
                          )}
                        </div>
                        <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                          {preset.description}
                        </p>
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </div>
            <span className="text-xs text-gray-400 dark:text-gray-500">
              Your preference is saved automatically
            </span>
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6">
          <div className="grid gap-6 md:grid-cols-2">
            {categories.map((category) => (
              <div key={category.name} className="space-y-3">
                <h3 className="text-sm font-semibold text-gray-900 dark:text-white uppercase tracking-wider">
                  {category.name}
                </h3>
                <div className="space-y-2">
                  {category.shortcuts.map((shortcut, index) => (
                    <div
                      key={shortcut.id || index}
                      className="flex items-center justify-between py-1.5"
                    >
                      <span className="text-sm text-gray-600 dark:text-gray-300">
                        {shortcut.description}
                      </span>
                      <kbd className="inline-flex items-center gap-1 px-2 py-1 text-xs font-mono font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded shadow-sm">
                        {formatShortcut(shortcut.keys, shortcut.isSequence)}
                      </kbd>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900/50">
          <p className="text-xs text-center text-gray-500 dark:text-gray-400">
            Press <kbd className="px-1.5 py-0.5 text-xs font-mono bg-gray-200 dark:bg-gray-700 rounded">?</kbd> anytime to show this help
            {' · '}
            <kbd className="px-1.5 py-0.5 text-xs font-mono bg-gray-200 dark:bg-gray-700 rounded">Esc</kbd> to close
          </p>
        </div>
      </div>
    </div>
  );
}
