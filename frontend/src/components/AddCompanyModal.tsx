import { useState } from 'react';
import { X } from 'lucide-react';

interface AddCompanyModalProps {
    isOpen: boolean;
    onClose: () => void;
    onSubmit: (data: CompanyData) => Promise<void>;
    initialData?: CompanyData | null;
}

export interface CompanyData {
    name: string;
    domain: string;
    plan: string;
    storageQuota: number;
    departments: string[];
}

export function AddCompanyModal({ isOpen, onClose, onSubmit, initialData }: AddCompanyModalProps) {
    const [formData, setFormData] = useState<CompanyData>({
        name: '',
        domain: '',
        plan: 'enterprise', // Default to enterprise hidden
        storageQuota: 1,
        departments: [],
    });
    const [newDept, setNewDept] = useState('');
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    // Populate form with initialData when it changes
    useState(() => {
        if (initialData) {
            setFormData(initialData);
        } else {
            setFormData({
                name: '',
                domain: '',
                plan: 'enterprise',
                storageQuota: 1,
                departments: [],
            });
        }
    });

    // Also update when isOpen changes to true, in case initialData changed
    if (isOpen && initialData && formData.name !== initialData.name && formData.domain !== initialData.domain) {
        setFormData(initialData);
    }

    const handleSubmit = async (e: React.FormEvent) => {
        e.preventDefault();
        setError(null);
        setIsSubmitting(true);

        try {
            await onSubmit(formData);
            // Reset form
            setFormData({
                name: '',
                domain: '',
                plan: 'enterprise',
                storageQuota: 1,
                departments: [],
            });
            onClose();
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to add company');
        } finally {
            setIsSubmitting(false);
        }
    };

    if (!isOpen) return null;

    return (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
            <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl max-w-md w-full mx-4">
                <div className="p-6 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
                    <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
                        {initialData ? 'Edit Company' : 'Add New Company'}
                    </h2>
                    <button
                        onClick={onClose}
                        className="p-1 hover:bg-gray-100 rounded-full transition-colors"
                    >
                        <X className="w-5 h-5 text-gray-500" />
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
                            Company Name <span className="text-red-500">*</span>
                        </label>
                        <input
                            type="text"
                            required
                            value={formData.name}
                            onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                            placeholder="Acme Corporation"
                            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400"
                        />
                    </div>

                    <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            Domain {initialData && <span className="text-xs text-gray-500 font-normal">(Cannot be changed)</span>} {!initialData && <span className="text-red-500">*</span>}
                        </label>
                        <input
                            type="text"
                            required
                            disabled={!!initialData}
                            value={formData.domain}
                            onChange={(e) => setFormData({ ...formData, domain: e.target.value })}
                            placeholder="acme.com"
                            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400 disabled:bg-gray-100 dark:disabled:bg-gray-800 disabled:text-gray-500"
                        />
                        {!initialData && <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">Company domain (used for branding and email)</p>}
                    </div>

                    {/* Plan selection removed, defaults to enterprise */}

                    <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                            Storage Quota (TB)
                        </label>
                        <input
                            type="number"
                            min="1"
                            max="100"
                            value={formData.storageQuota}
                            onChange={(e) => setFormData({ ...formData, storageQuota: parseInt(e.target.value) })}
                            className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white"
                        />
                        <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">Allocated storage limit</p>
                    </div>

                    {!initialData && (
                        <div>
                            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                                Departments
                            </label>
                            <div className="flex space-x-2 mb-2">
                                <input
                                    type="text"
                                    value={newDept}
                                    onChange={(e) => setNewDept(e.target.value)}
                                    placeholder="e.g. HR, Engineering"
                                    className="flex-1 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md focus:outline-none focus:ring-2 focus:ring-primary-500 bg-white dark:bg-gray-700 text-gray-900 dark:text-white placeholder-gray-500 dark:placeholder-gray-400"
                                    onKeyDown={(e) => {
                                        if (e.key === 'Enter') {
                                            e.preventDefault();
                                            if (newDept.trim()) {
                                                setFormData({
                                                    ...formData,
                                                    departments: [...formData.departments, newDept.trim()]
                                                });
                                                setNewDept('');
                                            }
                                        }
                                    }}
                                />
                                <button
                                    type="button"
                                    onClick={() => {
                                        if (newDept.trim()) {
                                            setFormData({
                                                ...formData,
                                                departments: [...formData.departments, newDept.trim()]
                                            });
                                            setNewDept('');
                                        }
                                    }}
                                    className="px-3 py-2 bg-gray-100 dark:bg-gray-700 text-gray-700 dark:text-gray-200 rounded-md hover:bg-gray-200 dark:hover:bg-gray-600"
                                >
                                    Add
                                </button>
                            </div>
                            <div className="flex flex-wrap gap-2">
                                {formData.departments.map((dept, index) => (
                                    <span key={index} className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-300">
                                        {dept}
                                        <button
                                            type="button"
                                            onClick={() => {
                                                const newDepts = [...formData.departments];
                                                newDepts.splice(index, 1);
                                                setFormData({ ...formData, departments: newDepts });
                                            }}
                                            className="ml-1.5 h-4 w-4 rounded-full inline-flex items-center justify-center hover:bg-blue-200 dark:hover:bg-blue-800 focus:outline-none"
                                        >
                                            <span className="sr-only">Remove</span>
                                            ×
                                        </button>
                                    </span>
                                ))}
                            </div>
                        </div>
                    )}

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
                            {isSubmitting ? (initialData ? 'Saving...' : 'Adding...') : (initialData ? 'Save Changes' : 'Add Company')}
                        </button>
                    </div>
                </form>
            </div>
        </div>
    );
}
