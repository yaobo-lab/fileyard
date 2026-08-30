import { X } from 'lucide-react';
import { SidebarItem } from '../context/ExtensionContext';
import { ExtensionIframe } from './ExtensionIframe';
import { ExtensionModule } from './ExtensionModule';

interface ExtensionPanelProps {
    item: SidebarItem;
    onClose: () => void;
}

export function ExtensionPanel({ item, onClose }: ExtensionPanelProps) {
    return (
        <div className="fixed inset-0 z-50 flex">
            {/* Backdrop */}
            <div 
                className="absolute inset-0 bg-black/30" 
                onClick={onClose}
            />
            
            {/* Panel */}
            <div className="relative ml-auto w-full max-w-2xl bg-white dark:bg-gray-800 shadow-xl flex flex-col">
                {/* Header */}
                <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                    <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                        {item.name}
                    </h2>
                    <button
                        onClick={onClose}
                        className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <X className="h-5 w-5" />
                    </button>
                </div>
                
                {/* Content */}
                <div className="flex-1 overflow-hidden">
                    {item.load_mode === 'iframe' ? (
                        <ExtensionIframe 
                            src={item.entrypoint} 
                            title={item.name}
                        />
                    ) : (
                        <ExtensionModule 
                            src={item.entrypoint}
                            extensionId={item.extension_id}
                        />
                    )}
                </div>
            </div>
        </div>
    );
}

