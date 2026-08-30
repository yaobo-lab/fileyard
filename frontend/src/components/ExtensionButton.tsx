import { useState } from 'react';
import { Puzzle } from 'lucide-react';
import clsx from 'clsx';
import { ButtonItem } from '../context/ExtensionContext';
import { ExtensionIframe } from './ExtensionIframe';
import { ExtensionModule } from './ExtensionModule';

interface ExtensionButtonProps {
    button: ButtonItem;
    variant?: 'default' | 'icon' | 'compact';
    className?: string;
}

/**
 * Renders an extension button that opens a modal with the extension content
 */
export function ExtensionButton({ button, variant = 'default', className = '' }: ExtensionButtonProps) {
    const [isOpen, setIsOpen] = useState(false);

    return (
        <>
            <button
                onClick={() => setIsOpen(true)}
                className={clsx(
                    "inline-flex items-center transition-colors",
                    variant === 'icon' 
                        ? "p-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-800"
                        : variant === 'compact'
                        ? "px-2 py-1 text-xs font-medium text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-700 rounded"
                        : "px-3 py-1.5 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg",
                    className
                )}
                title={button.name}
            >
                {button.icon ? (
                    <img src={button.icon} alt="" className="h-4 w-4" />
                ) : (
                    <Puzzle className="h-4 w-4" />
                )}
                {variant !== 'icon' && (
                    <span className="ml-1.5">{button.name}</span>
                )}
            </button>

            {/* Modal */}
            {isOpen && (
                <div className="fixed inset-0 z-50 overflow-y-auto">
                    <div className="flex min-h-full items-center justify-center p-4">
                        <div 
                            className="fixed inset-0 bg-black/50" 
                            onClick={() => setIsOpen(false)} 
                        />
                        <div className="relative bg-white dark:bg-gray-800 rounded-xl shadow-xl max-w-2xl w-full max-h-[80vh] flex flex-col">
                            {/* Header */}
                            <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                                <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                                    {button.name}
                                </h2>
                                <button
                                    onClick={() => setIsOpen(false)}
                                    className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                                >
                                    <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                                    </svg>
                                </button>
                            </div>
                            
                            {/* Content */}
                            <div className="flex-1 overflow-hidden min-h-[300px]">
                                {button.load_mode === 'iframe' ? (
                                    <ExtensionIframe 
                                        src={button.entrypoint} 
                                        title={button.name}
                                    />
                                ) : (
                                    <ExtensionModule 
                                        src={button.entrypoint}
                                        extensionId={button.extension_id}
                                    />
                                )}
                            </div>
                        </div>
                    </div>
                </div>
            )}
        </>
    );
}

interface ExtensionButtonGroupProps {
    buttons: ButtonItem[];
    location: string;
    className?: string;
}

/**
 * Renders a group of extension buttons filtered by location
 */
export function ExtensionButtonGroup({ buttons, location, className = '' }: ExtensionButtonGroupProps) {
    const filteredButtons = buttons.filter(b => b.location === location);
    
    if (filteredButtons.length === 0) {
        return null;
    }

    return (
        <div className={clsx("flex items-center gap-1", className)}>
            {filteredButtons.map((button) => (
                <ExtensionButton 
                    key={button.id} 
                    button={button} 
                    variant="compact"
                />
            ))}
        </div>
    );
}

