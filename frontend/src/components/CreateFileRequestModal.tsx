import { useState, useEffect } from 'react';
import { X, Folder, Plus, ChevronRight, ShieldAlert, Lock, Users, EyeOff } from 'lucide-react';
import { useAuthFetch } from '../context/AuthContext';
import { useSettings } from '../context/SettingsContext';

interface CreateFileRequestModalProps {
    isOpen: boolean;
    onClose: () => void;
    onSubmit: (data: FileRequestData) => Promise<void>;
    initialPath?: string;
    defaultVisibility?: 'department' | 'private';
}

export interface FileRequestData {
    name: string;
    destination_path: string;
    department_id?: string;
    expires_in_days: number;
    max_uploads?: number;
    visibility?: 'department' | 'private';
}

export function CreateFileRequestModal({ isOpen, onClose, onSubmit, initialPath = '/', currentCompanyId, defaultVisibility = 'department' }: CreateFileRequestModalProps & { currentCompanyId?: string }) {
    // console.log("VERSION 2 DEBUG: CreateFileRequestModal loaded");
    const { restrictions, complianceMode } = useSettings();
    const isPublicSharingBlocked = restrictions?.public_sharing_blocked || false;
    
    const [formData, setFormData] = useState<FileRequestData>({
        name: '',
        destination_path: initialPath,
        department_id: '',
        expires_in_days: 7,
        max_uploads: undefined,
        visibility: defaultVisibility,
    });
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [departments, setDepartments] = useState<any[]>([]);
    const [showFolderBrowser, setShowFolderBrowser] = useState(false);
    const [showNewFolderInput, setShowNewFolderInput] = useState(false);
    const [newFolderName, setNewFolderName] = useState('');
    const authFetch = useAuthFetch();

    useEffect(() => {
        if (isOpen) {
            fetchDepartments();
            setFormData(prev => ({ ...prev, destination_path: initialPath, visibility: defaultVisibility }));
        }
    }, [isOpen, initialPath, defaultVisibility]);

    const fetchDepartments = async () => {
        try {
            const response = await authFetch('/api/departments');
            if (response.ok) {
                const data = await response.json();
                setDepartments(data);
            }
        } catch (error) {
            console.error('Failed to fetch departments', error);
        }
    };

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setError(null);
        setIsSubmitting(true);

        try {
            await onSubmit(formData);
            // Reset form
            setFormData({
                name: '',
                destination_path: '/',
                department_id: '',
                expires_in_days: 7,
                max_uploads: undefined,
                visibility: defaultVisibility,
            });
            onClose();
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to create file request');
        } finally {
            setIsSubmitting(false);
        }
    };

    const handleCreateFolder = () => {
        if (newFolderName.trim()) {
            const newPath = formData.destination_path === '/'
                ? `/${newFolderName}`
                : `${formData.destination_path}/${newFolderName}`;
            setFormData({ ...formData, destination_path: newPath });
            setNewFolderName('');
            setShowNewFolderInput(false);
        }
    };

    const [quickFolders, setQuickFolders] = useState<{ path: string, label: string }[]>([
        { path: '/', label: 'Root' }
    ]);

    useEffect(() => {
        if (isOpen) {
            fetchFolders();
        }
    }, [isOpen]);

    const fetchFolders = async () => {
        if (!currentCompanyId) return;
        try {
            // Fetch root level folders
            const response = await authFetch(`/api/files/${currentCompanyId}?path=`);
            if (response.ok) {
                const files = await response.json();
                const folders = files
                    .filter((f: any) => f.is_dir)
                    .map((f: any) => ({
                        path: `/${f.name}`,
                        label: f.name
                    }));

                setQuickFolders([
                    { path: '/', label: 'Root' },
                    ...folders
                ]);
            }
        } catch (error) {
            console.error('Failed to fetch folders', error);
        }
    };

    if (!isOpen) return null;

    // Show compliance restriction message if public sharing is blocked
    if (isPublicSharingBlocked) {
        return (
            <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-md w-full mx-4">
                    <div className="p-6 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
                        <h2 className="text-xl font-semibold text-gray-900 dark:text-white flex items-center gap-2">
                            <ShieldAlert className="w-5 h-5 text-amber-500" />
                            Feature Restricted
                        </h2>
                        <button
                            onClick={onClose}
                            className="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-full transition-colors"
                        >
                            <X className="w-5 h-5 text-gray-500 dark:text-gray-400" />
                        </button>
                    </div>
                    <div className="p-6">
                        <div className="flex items-start gap-4 mb-6">
                            <div className="flex-shrink-0 w-12 h-12 bg-amber-100 dark:bg-amber-900/30 rounded-full flex items-center justify-center">
                                <Lock className="w-6 h-6 text-amber-600 dark:text-amber-400" />
                            </div>
                            <div>
                                <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-2">
                                    Public File Requests Disabled
                                </h3>
                                <p className="text-sm text-gray-600 dark:text-gray-400">
                                    Your organization has <strong>{complianceMode}</strong> compliance mode enabled, which restricts public file sharing for security reasons.
                                </p>
                            </div>
                        </div>
                        <div className="bg-gray-50 dark:bg-gray-700/50 rounded-lg p-4 mb-6">
                            <p className="text-xs text-gray-500 dark:text-gray-400">
                                Public file request links allow external users to upload files without authentication. This feature is disabled under {complianceMode} compliance to prevent unauthorized data access.
                            </p>
                        </div>
                        <div className="flex justify-end">
                            <button
                                onClick={onClose}
                                className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-200 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-50 dark:hover:bg-gray-600"
                            >
                                Close
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        );
    }

    return (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-md w-full mx-4">
                <div className="p-6 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
                    <h2 className="text-xl font-semibold text-gray-900 dark:text-white">Create File Request</h2>
                    <button
                        onClick={onClose}
                        className="p-1 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-full transition-colors"
                    >
                        <X className="w-5 h-5 text-gray-500 dark:text-gray-400" />
                    </button>
                </div>

                <form onSubmit={handleSubmit} className="p-6 space-y-4">
                    {error && (
                        <div className="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-md text-sm text-red-600 dark:text-red-400">
                            {error}
                        </div>
                    )}

                    <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            Request Name <span className="text-red-500">*</span>
                        </label>
                        <input
                            type="text"
                            required
                            value={formData.name}
                            onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                            placeholder="Q4 Financial Reports"
                            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400"
                        />
                        <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">A descriptive name for this upload request</p>
                    </div>

                    {/* Visibility Selector */}
                    <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            Visibility
                        </label>
                        <div className="flex gap-3">
                            <button
                                type="button"
                                onClick={() => setFormData({ ...formData, visibility: 'department' })}
                                className={`flex-1 flex items-center justify-center px-4 py-2.5 border rounded-lg text-sm font-medium transition-all ${
                                    formData.visibility === 'department'
                                        ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20 text-blue-700 dark:text-blue-300 ring-1 ring-blue-500'
                                        : 'border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-600'
                                }`}
                            >
                                <Users className="w-4 h-4 mr-2" />
                                Department
                            </button>
                            <button
                                type="button"
                                onClick={() => setFormData({ ...formData, visibility: 'private' })}
                                className={`flex-1 flex items-center justify-center px-4 py-2.5 border rounded-lg text-sm font-medium transition-all ${
                                    formData.visibility === 'private'
                                        ? 'border-purple-500 bg-purple-50 dark:bg-purple-900/20 text-purple-700 dark:text-purple-300 ring-1 ring-purple-500'
                                        : 'border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-600'
                                }`}
                            >
                                <EyeOff className="w-4 h-4 mr-2" />
                                Private
                            </button>
                        </div>
                        <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
                            {formData.visibility === 'department' 
                                ? 'Visible to all members in your department'
                                : 'Only visible to you'}
                        </p>
                    </div>

                    {departments.length > 0 && formData.visibility === 'department' && (
                        <div>
                            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                Department
                            </label>
                            <select
                                value={formData.department_id || ''}
                                onChange={(e) => setFormData({ ...formData, department_id: e.target.value || undefined })}
                                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                            >
                                <option value="">All Departments</option>
                                {departments.map((dept) => (
                                    <option key={dept.id} value={dept.id}>{dept.name}</option>
                                ))}
                            </select>
                            <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">Optional: Restrict access to a specific department</p>
                        </div>
                    )}

                    <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            Destination Folder <span className="text-red-500">*</span>
                        </label>
                        <div className="relative">
                            <input
                                type="text"
                                required
                                value={formData.destination_path}
                                onChange={(e) => setFormData({ ...formData, destination_path: e.target.value })}
                                placeholder="/Finance/2024"
                                className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 pr-10"
                                onFocus={() => setShowFolderBrowser(true)}
                            />
                            <Folder className="absolute right-3 top-2.5 w-5 h-5 text-gray-400 dark:text-gray-500 pointer-events-none" />
                        </div>

                        {showFolderBrowser && (
                            <div className="mt-2 p-3 border border-gray-200 dark:border-gray-600 rounded-md bg-gray-50 dark:bg-gray-700/50 max-h-48 overflow-y-auto">
                                <div className="flex items-center justify-between mb-2">
                                    <span className="text-xs font-medium text-gray-700 dark:text-gray-300">Quick Select</span>
                                    <button
                                        type="button"
                                        onClick={() => setShowNewFolderInput(!showNewFolderInput)}
                                        className="text-xs text-primary-600 dark:text-primary-400 hover:underline flex items-center"
                                    >
                                        <Plus className="w-3 h-3 mr-1" />
                                        New Folder
                                    </button>
                                </div>

                                {showNewFolderInput && (
                                    <div className="mb-2 flex gap-2">
                                        <input
                                            type="text"
                                            value={newFolderName}
                                            onChange={(e) => setNewFolderName(e.target.value)}
                                            placeholder="Folder name"
                                            className="flex-1 px-2 py-1 text-sm border border-gray-300 dark:border-gray-600 rounded bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                                            onKeyPress={(e) => e.key === 'Enter' && (e.preventDefault(), handleCreateFolder())}
                                        />
                                        <button
                                            type="button"
                                            onClick={handleCreateFolder}
                                            className="px-3 py-1 text-sm bg-primary-600 text-white rounded hover:bg-primary-700"
                                        >
                                            Create
                                        </button>
                                    </div>
                                )}

                                <div className="space-y-1">
                                    {quickFolders.map((folder) => (
                                        <button
                                            key={folder.path}
                                            type="button"
                                            onClick={() => {
                                                setFormData({ ...formData, destination_path: folder.path });
                                                setShowFolderBrowser(false);
                                            }}
                                            className="w-full text-left px-2 py-1.5 text-sm rounded hover:bg-gray-200 dark:hover:bg-gray-600 flex items-center text-gray-700 dark:text-gray-300"
                                        >
                                            <Folder className="w-4 h-4 mr-2 text-primary-600 dark:text-primary-400" />
                                            {folder.label}
                                            <ChevronRight className="w-3 h-3 ml-auto text-gray-400" />
                                        </button>
                                    ))}
                                </div>
                            </div>
                        )}
                        <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">Where uploaded files will be stored</p>
                    </div>

                    <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            Expires In (Days) <span className="text-red-500">*</span>
                        </label>
                        <input
                            type="number"
                            required
                            min="1"
                            max="365"
                            value={formData.expires_in_days}
                            onChange={(e) => setFormData({ ...formData, expires_in_days: parseInt(e.target.value) })}
                            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                        />
                        <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">Link will expire after this many days (1-365)</p>
                    </div>

                    <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            Maximum Uploads (Optional)
                        </label>
                        <input
                            type="number"
                            min="1"
                            value={formData.max_uploads || ''}
                            onChange={(e) => setFormData({
                                ...formData,
                                max_uploads: e.target.value ? parseInt(e.target.value) : undefined
                            })}
                            placeholder="No limit"
                            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400"
                        />
                        <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">Leave empty for unlimited uploads</p>
                    </div>

                    <div className="pt-4 flex justify-end space-x-3">
                        <button
                            type="button"
                            onClick={onClose}
                            className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-200 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md hover:bg-gray-50 dark:hover:bg-gray-600"
                        >
                            Cancel
                        </button>
                        <button
                            type="submit"
                            disabled={isSubmitting}
                            className="px-4 py-2 text-sm font-medium text-white bg-primary-600 rounded-md hover:bg-primary-700 disabled:opacity-50 disabled:cursor-not-allowed"
                        >
                            {isSubmitting ? 'Creating...' : 'Create Request'}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
}
