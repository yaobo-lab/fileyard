import { useEffect, useCallback, useRef } from 'react';

export interface Shortcut {
  id: string;
  keys: string[]; // e.g., ['g', 'd'] for sequence, ['ctrl', 'k'] for combo
  description: string;
  category: 'navigation' | 'files' | 'ui' | 'selection';
  action: () => void;
  enabled?: boolean;
  isSequence?: boolean; // true for 'g' then 'd', false for 'ctrl+k'
}

interface UseKeyboardShortcutsOptions {
  enabled?: boolean;
  sequenceTimeout?: number; // ms to wait for next key in sequence
}

// Check if the event target is an input element where shortcuts should be disabled
function isInputElement(target: EventTarget | null): boolean {
  if (!target || !(target instanceof HTMLElement)) return false;
  
  const tagName = target.tagName.toLowerCase();
  if (tagName === 'input' || tagName === 'textarea' || tagName === 'select') {
    return true;
  }
  
  // Check for contenteditable
  if (target.isContentEditable) {
    return true;
  }
  
  return false;
}

// Get platform-aware modifier key
export function getModifierKey(): 'meta' | 'ctrl' {
  return navigator.platform.toLowerCase().includes('mac') ? 'meta' : 'ctrl';
}

// Format shortcut for display
export function formatShortcut(keys: string[], isSequence?: boolean): string {
  const modifierKey = getModifierKey();
  
  const formatKey = (key: string): string => {
    switch (key.toLowerCase()) {
      case 'meta':
      case 'ctrl':
        return modifierKey === 'meta' ? '⌘' : 'Ctrl';
      case 'shift':
        return modifierKey === 'meta' ? '⇧' : 'Shift';
      case 'alt':
        return modifierKey === 'meta' ? '⌥' : 'Alt';
      case 'enter':
        return '↵';
      case 'escape':
        return 'Esc';
      case 'backspace':
        return '⌫';
      case 'delete':
        return 'Del';
      case 'arrowup':
        return '↑';
      case 'arrowdown':
        return '↓';
      case 'arrowleft':
        return '←';
      case 'arrowright':
        return '→';
      case 'space':
        return 'Space';
      case '/':
        return '/';
      case '?':
        return '?';
      default:
        return key.toUpperCase();
    }
  };
  
  if (isSequence) {
    return keys.map(formatKey).join(' then ');
  }
  
  return keys.map(formatKey).join(modifierKey === 'meta' ? '' : '+');
}

// Normalize key from event
function normalizeKey(event: KeyboardEvent): string {
  const key = event.key.toLowerCase();
  
  // Handle special keys
  if (key === ' ') return 'space';
  if (key === 'backspace') return 'backspace';
  if (key === 'delete') return 'delete';
  
  return key;
}

export function useKeyboardShortcuts(
  shortcuts: Shortcut[],
  options: UseKeyboardShortcutsOptions = {}
) {
  const { enabled = true, sequenceTimeout = 1000 } = options;
  
  const sequenceBuffer = useRef<string[]>([]);
  const sequenceTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  
  const clearSequence = useCallback(() => {
    sequenceBuffer.current = [];
    if (sequenceTimer.current) {
      clearTimeout(sequenceTimer.current);
      sequenceTimer.current = null;
    }
  }, []);
  
  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    if (!enabled) return;
    
    // Don't handle shortcuts when typing in input fields
    if (isInputElement(event.target)) {
      // Exception: Escape should always work to blur/close
      if (event.key === 'Escape') {
        const target = event.target as HTMLElement;
        target.blur();
      }
      return;
    }
    
    const key = normalizeKey(event);
    const modifierKey = getModifierKey();
    const hasModifier = modifierKey === 'meta' ? event.metaKey : event.ctrlKey;
    const hasShift = event.shiftKey;
    const hasAlt = event.altKey;
    
    // Build current key combo
    const currentKeys: string[] = [];
    if (hasModifier) currentKeys.push(modifierKey);
    if (hasShift) currentKeys.push('shift');
    if (hasAlt) currentKeys.push('alt');
    currentKeys.push(key);
    
    // Check for modifier combos first (non-sequence shortcuts)
    for (const shortcut of shortcuts) {
      if (!shortcut.enabled && shortcut.enabled !== undefined) continue;
      if (shortcut.isSequence) continue;
      
      const shortcutKeys = shortcut.keys.map(k => k.toLowerCase());
      
      // Check if all keys match
      if (shortcutKeys.length === currentKeys.length &&
          shortcutKeys.every(k => currentKeys.includes(k))) {
        event.preventDefault();
        shortcut.action();
        clearSequence();
        return;
      }
    }
    
    // Handle sequences (no modifiers for sequence keys)
    if (!hasModifier && !hasAlt) {
      // Add to sequence buffer
      sequenceBuffer.current.push(key);
      
      // Reset timer
      if (sequenceTimer.current) {
        clearTimeout(sequenceTimer.current);
      }
      sequenceTimer.current = setTimeout(clearSequence, sequenceTimeout);
      
      // Check for matching sequences
      for (const shortcut of shortcuts) {
        if (!shortcut.enabled && shortcut.enabled !== undefined) continue;
        if (!shortcut.isSequence) continue;
        
        const shortcutKeys = shortcut.keys.map(k => k.toLowerCase());
        const bufferStr = sequenceBuffer.current.join(',');
        const shortcutStr = shortcutKeys.join(',');
        
        // Exact match
        if (bufferStr === shortcutStr) {
          event.preventDefault();
          shortcut.action();
          clearSequence();
          return;
        }
        
        // Partial match - keep waiting
        if (shortcutStr.startsWith(bufferStr + ',')) {
          event.preventDefault();
          return;
        }
      }
      
      // Check for single-key shortcuts (like 'u' for upload, '?' for help)
      for (const shortcut of shortcuts) {
        if (!shortcut.enabled && shortcut.enabled !== undefined) continue;
        if (shortcut.isSequence) continue;
        if (shortcut.keys.length !== 1) continue;
        
        // Allow shift for character keys that require it (like ? ! @ # etc.)
        const isShiftedChar = ['?', '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_', '+', '{', '}', '|', ':', '"', '<', '>', '~'].includes(key);
        if (shortcut.keys[0].toLowerCase() === key && (!hasShift || isShiftedChar)) {
          event.preventDefault();
          shortcut.action();
          clearSequence();
          return;
        }
      }
      
      // No match found, clear if buffer is getting long
      if (sequenceBuffer.current.length > 3) {
        clearSequence();
      }
    }
  }, [enabled, shortcuts, sequenceTimeout, clearSequence]);
  
  useEffect(() => {
    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      clearSequence();
    };
  }, [handleKeyDown, clearSequence]);
  
  return { clearSequence };
}

