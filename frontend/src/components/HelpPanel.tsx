import { X, Shield, Users, Briefcase, User } from 'lucide-react';

interface HelpPanelProps {
    isOpen: boolean;
    onClose: () => void;
}

export function HelpPanel({ isOpen, onClose }: HelpPanelProps) {
    if (!isOpen) return null;

    const roles = [
        {
            name: 'SuperAdmin',
            icon: Shield,
            color: 'text-purple-600 dark:text-purple-300',
            bgColor: 'bg-purple-50 dark:bg-purple-900/20',
            permissions: [
                'Manage all companies/tenants',
                'Create and delete companies',
                'Access all company data',
                'Manage system settings',
                'Full administrative control'
            ]
        },
        {
            name: 'Admin',
            icon: Users,
            color: 'text-blue-600 dark:text-blue-300',
            bgColor: 'bg-blue-50 dark:bg-blue-900/20',
            permissions: [
                'Manage company users',
                'Create file requests',
                'View all company files',
                'Manage company settings',
                'Cannot manage other companies'
            ]
        },
        {
            name: 'Manager',
            icon: Briefcase,
            color: 'text-green-600 dark:text-green-300',
            bgColor: 'bg-green-50 dark:bg-green-900/20',
            permissions: [
                'Create file requests',
                'View team files',
                'Upload and download files',
                'Share files with team',
                'Cannot manage users'
            ]
        },
        {
            name: 'Employee',
            icon: User,
            color: 'text-gray-600 dark:text-gray-300',
            bgColor: 'bg-gray-50 dark:bg-gray-700',
            permissions: [
                'Upload files via requests',
                'View assigned files',
                'Download shared files',
                'Basic file operations',
                'Limited access'
            ]
        }
    ];

    return (
        <>
            {/* Backdrop */}
            <div
                className="fixed inset-0 bg-black/50 z-40 transition-opacity"
                onClick={onClose}
            />

            {/* Panel */}
            <div className="fixed right-0 top-0 h-full w-full max-w-md bg-white dark:bg-gray-800 shadow-2xl z-50 overflow-y-auto">
                {/* Header */}
                <div className="sticky top-0 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex items-center justify-between">
                    <h2 className="text-xl font-semibold text-gray-900 dark:text-white">User Roles & Permissions</h2>
                    <button
                        onClick={onClose}
                        className="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                    >
                        <X className="w-5 h-5 text-gray-500 dark:text-gray-400" />
                    </button>
                </div>

                {/* Content */}
                <div className="p-6 space-y-4">
                    {roles.map((role) => (
                        <div
                            key={role.name}
                            className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:shadow-md transition-shadow"
                        >
                            <div className="flex items-start gap-3 mb-3">
                                <div className={`p-2 rounded-lg ${role.bgColor}`}>
                                    <role.icon className={`w-5 h-5 ${role.color}`} />
                                </div>
                                <div>
                                    <h3 className={`font-semibold ${role.color}`}>{role.name}</h3>
                                </div>
                            </div>
                            <ul className="space-y-2">
                                {role.permissions.map((permission, idx) => (
                                    <li key={idx} className="flex items-start gap-2 text-sm text-gray-600 dark:text-gray-300">
                                        <span className="text-gray-400 dark:text-gray-500 mt-0.5">•</span>
                                        <span>{permission}</span>
                                    </li>
                                ))}
                            </ul>
                        </div>
                    ))}
                </div>

                {/* Footer */}
                <div className="sticky bottom-0 bg-gray-50 dark:bg-gray-900/50 border-t border-gray-200 dark:border-gray-700 px-6 py-4">
                    <p className="text-sm text-gray-600 dark:text-gray-400 text-center">
                        Contact your administrator to change your role
                    </p>
                </div>
            </div>
        </>
    );
}
