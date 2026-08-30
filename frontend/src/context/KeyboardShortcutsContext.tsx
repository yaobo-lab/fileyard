import React, { createContext, useContext, useState, useCallback, useEffect, ReactNode, useMemo, useRef } from 'react';
import { formatShortcut } from '../hooks/useKeyboardShortcuts';
import {
  ShortcutPresetId,
  ShortcutBinding,
  PRESETS,
  getPreset,
  mergeWithCustomBindings,
  getResolvedShortcuts,
  SHORTCUT_ACTIONS,
  ShortcutActionId,
  resolveMod,
} from '../hooks/shortcutPresets';

// localStorage keys (used as fallback)
const STORAGE_KEY_PRESET = 'keyboard-shortcuts-preset';

interface ShortcutCategory {
  name: string;
  shortcuts: Array<{
    id?: string;
    keys: string[];
    description: string;
    isSequence?: boolean;
  }>;
}

interface KeyboardShortcutsContextType {
  // Help modal
  isHelpOpen: boolean;
  openHelp: () => void;
  closeHelp: () => void;
  toggleHelp: () => void;
  
  // Categories for display
  categories: ShortcutCategory[];
  
  // Preset management
  currentPresetId: ShortcutPresetId;
  presets: typeof PRESETS;
  setPreset: (presetId: ShortcutPresetId) => void;
  
  // Get current binding for an action
  getBinding: (actionId: ShortcutActionId) => ShortcutBinding | undefined;
  getResolvedBinding: (actionId: ShortcutActionId) => { keys: string[]; isSequence?: boolean } | undefined;
}

const KeyboardShortcutsContext = createContext<KeyboardShortcutsContextType | null>(null);

interface KeyboardShortcutsProviderProps {
  children: ReactNode;
}

// Helper to get auth token from localStorage
function getAuthToken(): string | null {
  return localStorage.getItem('token');
}

// API functions for user preferences
async function fetchUserPreferences(): Promise<{ keyboard_shortcut_preset?: ShortcutPresetId }> {
  const token = getAuthToken();
  if (!token) return {};
  
  try {
    const response = await fetch('/api/users/me/preferences', {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    });
    if (response.ok) {
      return await response.json();
    }
  } catch (error) {
    console.error('Failed to fetch user preferences:', error);
  }
  return {};
}

async function saveUserPreferences(settings: { keyboard_shortcut_preset: ShortcutPresetId }): Promise<void> {
  const token = getAuthToken();
  if (!token) return;
  
  try {
    await fetch('/api/users/me/preferences', {
      method: 'PUT',
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ settings }),
    });
  } catch (error) {
    console.error('Failed to save user preferences:', error);
  }
}

export function KeyboardShortcutsProvider({ children }: KeyboardShortcutsProviderProps) {
  const [isHelpOpen, setIsHelpOpen] = useState(false);
  const [isLoaded, setIsLoaded] = useState(false);
  const saveTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  
  // Load initial state from localStorage (will be overwritten by API)
  const [currentPresetId, setCurrentPresetId] = useState<ShortcutPresetId>(() => {
    if (typeof window === 'undefined') return 'default';
    const stored = localStorage.getItem(STORAGE_KEY_PRESET);
    if (stored && (stored === 'default' || stored === 'vim' || stored === 'minimal')) {
      return stored;
    }
    return 'default';
  });

  // Fetch preset from API on mount
  useEffect(() => {
    let mounted = true;
    
    const loadPreferences = async () => {
      const prefs = await fetchUserPreferences();
      if (mounted && prefs.keyboard_shortcut_preset) {
        const preset = prefs.keyboard_shortcut_preset;
        if (preset === 'default' || preset === 'vim' || preset === 'minimal') {
          setCurrentPresetId(preset);
          // Also update localStorage
          localStorage.setItem(STORAGE_KEY_PRESET, preset);
        }
      }
      if (mounted) setIsLoaded(true);
    };
    
    loadPreferences();
    
    return () => {
      mounted = false;
    };
  }, []);

  // Get merged bindings (just from preset now, no custom bindings)
  const mergedBindings = useMemo(() => {
    const preset = getPreset(currentPresetId);
    return preset.shortcuts;
  }, [currentPresetId]);

  const openHelp = useCallback(() => setIsHelpOpen(true), []);
  const closeHelp = useCallback(() => setIsHelpOpen(false), []);
  const toggleHelp = useCallback(() => setIsHelpOpen(prev => !prev), []);
  
  const setPreset = useCallback((presetId: ShortcutPresetId) => {
    setCurrentPresetId(presetId);
    
    // Save to localStorage immediately
    localStorage.setItem(STORAGE_KEY_PRESET, presetId);
    
    // Debounce API save
    if (saveTimeoutRef.current) {
      clearTimeout(saveTimeoutRef.current);
    }
    saveTimeoutRef.current = setTimeout(() => {
      saveUserPreferences({ keyboard_shortcut_preset: presetId });
    }, 500);
  }, []);
  
  // Get binding for a specific action
  const getBinding = useCallback((actionId: ShortcutActionId): ShortcutBinding | undefined => {
    return mergedBindings[actionId];
  }, [mergedBindings]);
  
  // Get resolved binding (with 'mod' converted to actual key)
  const getResolvedBinding = useCallback((actionId: ShortcutActionId) => {
    const binding = mergedBindings[actionId];
    if (!binding) return undefined;
    return {
      keys: resolveMod(binding.keys),
      isSequence: binding.isSequence,
    };
  }, [mergedBindings]);
  
  // Build categories for display in help modal
  const categories = useMemo((): ShortcutCategory[] => {
    const resolvedShortcuts = getResolvedShortcuts(mergedBindings);
    
    const categoryMap: Record<string, ShortcutCategory> = {
      navigation: { name: 'Navigation', shortcuts: [] },
      ui: { name: 'UI Controls', shortcuts: [] },
      files: { name: 'File Operations', shortcuts: [] },
      selection: { name: 'Selection', shortcuts: [] },
    };
    
    for (const shortcut of resolvedShortcuts) {
      const cat = categoryMap[shortcut.category];
      if (cat) {
        cat.shortcuts.push({
          id: shortcut.id,
          keys: shortcut.binding.keys,
          description: shortcut.description,
          isSequence: shortcut.binding.isSequence,
        });
      }
    }
    
    // Return categories in order, excluding empty ones
    return ['navigation', 'ui', 'files', 'selection']
      .map(key => categoryMap[key])
      .filter(cat => cat.shortcuts.length > 0);
  }, [mergedBindings]);

  return (
    <KeyboardShortcutsContext.Provider
      value={{
        isHelpOpen,
        openHelp,
        closeHelp,
        toggleHelp,
        categories,
        currentPresetId,
        presets: PRESETS,
        setPreset,
        getBinding,
        getResolvedBinding,
      }}
    >
      {children}
    </KeyboardShortcutsContext.Provider>
  );
}

export function useKeyboardShortcutsContext() {
  const context = useContext(KeyboardShortcutsContext);
  if (!context) {
    throw new Error('useKeyboardShortcutsContext must be used within a KeyboardShortcutsProvider');
  }
  return context;
}

export { formatShortcut };
