import { Keyboard, Check } from 'lucide-react';
import { useKeyboardShortcutsContext, formatShortcut } from '../../context/KeyboardShortcutsContext';
import { SHORTCUT_ACTIONS, ShortcutActionId, ShortcutPresetId, resolveMod } from '../../hooks/shortcutPresets';
import clsx from 'clsx';

export function KeyboardShortcutsSettings() {
    const { 
        currentPresetId, 
        presets, 
        setPreset,
        getResolvedBinding,
    } = useKeyboardShortcutsContext();
    
    const currentPreset = presets[currentPresetId];

    // Group shortcuts by category
    const shortcutsByCategory = Object.entries(SHORTCUT_ACTIONS).reduce((acc, [id, action]) => {
        if (!acc[action.category]) {
            acc[action.category] = [];
        }
        acc[action.category].push({ id, ...action });
        return acc;
    }, {} as Record<string, Array<{ id: string; description: string; category: string }>>);

    const categoryLabels: Record<string, string> = {
        navigation: 'Navigation',
        ui: 'UI Controls',
        files: 'File Operations',
        selection: 'Selection',
    };

    return (
        <div className="space-y-6">
            <div>
                <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Keyboard Shortcuts</h2>
                <p className="text-sm text-gray-500 dark:text-gray-400">Choose a shortcut preset that matches your workflow</p>
            </div>

            {/* Preset Selection */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="font-medium text-gray-900 dark:text-white flex items-center gap-2">
                        <Keyboard className="w-5 h-5 text-gray-500" />
                        Shortcut Preset
                    </h3>
                    <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
                        Presets change all shortcuts to match a specific workflow style
                    </p>
                </div>
                <div className="p-4 space-y-3">
                    {Object.values(presets).map((preset) => (
                        <label
                            key={preset.id}
                            className={clsx(
                                "flex items-start gap-4 p-4 rounded-lg border-2 cursor-pointer transition-all",
                                currentPresetId === preset.id
                                    ? "border-primary-500 bg-primary-50 dark:bg-primary-900/20"
                                    : "border-transparent bg-gray-50 dark:bg-gray-700/50 hover:bg-gray-100 dark:hover:bg-gray-700"
                            )}
                        >
                            <input
                                type="radio"
                                name="preset"
                                value={preset.id}
                                checked={currentPresetId === preset.id}
                                onChange={() => setPreset(preset.id as ShortcutPresetId)}
                                className="mt-1 w-4 h-4 text-primary-600 focus:ring-primary-500"
                            />
                            <div className="flex-1">
                                <div className="flex items-center gap-2">
                                    <p className="text-sm font-medium text-gray-900 dark:text-white">{preset.name}</p>
                                    {currentPresetId === preset.id && (
                                        <Check className="w-4 h-4 text-primary-500" />
                                    )}
                                </div>
                                <p className="text-sm text-gray-500 dark:text-gray-400 mt-0.5">{preset.description}</p>
                            </div>
                        </label>
                    ))}
                </div>
            </div>

            {/* Shortcuts Reference */}
            <div className="space-y-4">
                <h3 className="text-sm font-semibold text-gray-900 dark:text-white uppercase tracking-wider">
                    {currentPreset.name} Preset Shortcuts
                </h3>
                
                {['navigation', 'ui', 'files', 'selection'].map((category) => {
                    const shortcuts = shortcutsByCategory[category];
                    if (!shortcuts || shortcuts.length === 0) return null;

                    return (
                        <div key={category} className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                            <div className="px-6 py-3 border-b border-gray-200 dark:border-gray-700">
                                <h4 className="font-medium text-gray-900 dark:text-white">{categoryLabels[category]}</h4>
                            </div>
                            <div className="divide-y divide-gray-100 dark:divide-gray-700">
                                {shortcuts.map((shortcut) => {
                                    const resolvedBinding = getResolvedBinding(shortcut.id as ShortcutActionId);

                                    return (
                                        <div
                                            key={shortcut.id}
                                            className="flex items-center justify-between px-6 py-3"
                                        >
                                            <span className="text-sm text-gray-700 dark:text-gray-300">
                                                {shortcut.description}
                                            </span>
                                            <kbd className="inline-flex items-center gap-1 px-2.5 py-1 text-xs font-mono font-medium text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded shadow-sm">
                                                {resolvedBinding ? formatShortcut(resolvedBinding.keys, resolvedBinding.isSequence) : '—'}
                                            </kbd>
                                        </div>
                                    );
                                })}
                            </div>
                        </div>
                    );
                })}
            </div>

            {/* Tips */}
            <div className="bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200 dark:border-gray-700 p-4">
                <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2">Tips</h4>
                <ul className="text-xs text-gray-500 dark:text-gray-400 space-y-1 list-disc list-inside">
                    <li>Press <kbd className="px-1 py-0.5 bg-gray-200 dark:bg-gray-700 rounded text-xs">?</kbd> anywhere to see the shortcuts help modal</li>
                    <li>Shortcuts are disabled when typing in input fields</li>
                    <li>Your preset preference is saved in your browser</li>
                    <li>Vim preset uses j/k/h/l for navigation like in Vim</li>
                </ul>
            </div>
        </div>
    );
}
