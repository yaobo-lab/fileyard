import { useState, useEffect } from 'react';
import { useAuth, useAuthFetch } from '../context/AuthContext';
import { useTenant } from '../context/TenantContext';
import { Folder, FileText, Image, Film, MoreVertical, Download, Trash2, Search, Filter, ChevronRight, Home } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';

interface FileItem {
    id: string;
    name: string;
    type: 'folder' | 'document' | 'image' | 'video';
    size: string;
    modified: string;
    owner: string;
    department_id?: string;
}

interface Department {
    id: string;
    name: string;
}

export function Files() {
    const [files, setFiles] = useState<FileItem[]>([]);
    const [departments, setDepartments] = useState<Department[]>([]);
    const [selectedDepartment, setSelectedDepartment] = useState<string>('');
    const [currentPath, setCurrentPath] = useState<string>('/');
    const [isLoading, setIsLoading] = useState(true);
    const [searchQuery, setSearchQuery] = useState('');

    const { user } = useAuth();
    const { currentCompany } = useTenant();
    const authFetch = useAuthFetch();

    const isAdmin = user?.role === 'SuperAdmin' || user?.role === 'Admin';

    useEffect(() => {
        if (isAdmin) {
            fetchDepartments();
        }
        fetchFiles();
    }, [currentCompany.id, selectedDepartment, currentPath]);

    const fetchDepartments = async () => {
        try {
            const res = await authFetch('/api/departments');
            if (res.ok) {
                const data = await res.json();
                setDepartments(data);
            }
        } catch (error) {
            console.error('Failed to fetch departments', error);
        }
    };

    const fetchFiles = async () => {
        setIsLoading(true);
        try {
            let url = `/api/files/${currentCompany.id}?path=${encodeURIComponent(currentPath)}`;
            if (selectedDepartment) {
                url += `&department_id=${selectedDepartment}`;
            }

            const res = await authFetch(url);
            if (res.ok) {
                const data = await res.json();
                setFiles(data);
            }
        } catch (error) {
            console.error('Failed to fetch files', error);
        } finally {
            setIsLoading(false);
        }
    };

    const getFileIcon = (type: string) => {
        switch (type) {
            case 'folder': return <Folder className="w-5 h-5 text-yellow-500" />;
            case 'image': return <Image className="w-5 h-5 text-purple-500" />;
            case 'video': return <Film className="w-5 h-5 text-red-500" />;
            default: return <FileText className="w-5 h-5 text-blue-500" />;
        }
    };

    const filteredFiles = files.filter(file =>
        file.name.toLowerCase().includes(searchQuery.toLowerCase())
    );

    return (
        <div className="space-y-6">
            <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
                <h1 className="text-2xl font-bold text-gray-900 dark:text-white">Files</h1>

                <div className="flex items-center gap-3 w-full sm:w-auto">
                    {isAdmin && (
                        <select
                            value={selectedDepartment}
                            onChange={(e) => setSelectedDepartment(e.target.value)}
                            className="px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
                        >
                            <option value="">All Departments</option>
                            {departments.map(dept => (
                                <option key={dept.id} value={dept.id}>{dept.name}</option>
                            ))}
                        </select>
                    )}
                    <button className="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 transition-colors text-sm font-medium">
                        Upload File
                    </button>
                </div>
            </div>

            {/* Breadcrumbs & Search */}
            <div className="flex flex-col sm:flex-row justify-between items-center gap-4 bg-white dark:bg-gray-800 p-4 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700">
                <div className="flex items-center text-sm text-gray-600 dark:text-gray-400">
                    <button
                        onClick={() => setCurrentPath('/')}
                        className="hover:text-primary-600 dark:hover:text-primary-400 flex items-center"
                    >
                        <Home className="w-4 h-4 mr-1" />
                        Home
                    </button>
                    {currentPath !== '/' && currentPath.split('/').filter(Boolean).map((part, index, arr) => (
                        <div key={index} className="flex items-center">
                            <ChevronRight className="w-4 h-4 mx-1 text-gray-400" />
                            <span className="font-medium text-gray-900 dark:text-white">{part}</span>
                        </div>
                    ))}
                </div>

                <div className="relative w-full sm:w-64">
                    <Search className="absolute left-3 top-2.5 w-4 h-4 text-gray-400" />
                    <input
                        type="text"
                        placeholder="Search files..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        className="w-full pl-9 pr-4 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-700 text-sm focus:outline-none focus:ring-2 focus:ring-primary-500"
                    />
                </div>
            </div>

            {/* File List */}
            <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 overflow-hidden">
                <div className="overflow-x-auto">
                    <table className="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                        <thead className="bg-gray-50 dark:bg-gray-700/50">
                            <tr>
                                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Name</th>
                                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Size</th>
                                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Modified</th>
                                <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Owner</th>
                                <th className="relative px-6 py-3"><span className="sr-only">Actions</span></th>
                            </tr>
                        </thead>
                        <tbody className="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
                            {isLoading ? (
                                <tr>
                                    <td colSpan={5} className="px-6 py-12 text-center text-gray-500 dark:text-gray-400">
                                        Loading files...
                                    </td>
                                </tr>
                            ) : filteredFiles.length === 0 ? (
                                <tr>
                                    <td colSpan={5} className="px-6 py-12 text-center text-gray-500 dark:text-gray-400">
                                        <div className="flex flex-col items-center justify-center">
                                            <Folder className="w-12 h-12 text-gray-300 dark:text-gray-600 mb-3" />
                                            <p className="text-lg font-medium text-gray-900 dark:text-white">No files found</p>
                                            <p className="text-sm text-gray-500">Upload a file to get started</p>
                                        </div>
                                    </td>
                                </tr>
                            ) : (
                                filteredFiles.map((file) => (
                                    <tr key={file.id} className="hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors group">
                                        <td className="px-6 py-4 whitespace-nowrap">
                                            <div className="flex items-center">
                                                <div className="flex-shrink-0 h-10 w-10 flex items-center justify-center rounded-lg bg-gray-100 dark:bg-gray-700">
                                                    {getFileIcon(file.type)}
                                                </div>
                                                <div className="ml-4">
                                                    <div className="text-sm font-medium text-gray-900 dark:text-white">{file.name}</div>
                                                </div>
                                            </div>
                                        </td>
                                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                            {file.size}
                                        </td>
                                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                            {formatDistanceToNow(new Date(file.modified), { addSuffix: true })}
                                        </td>
                                        <td className="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                                            {file.owner}
                                        </td>
                                        <td className="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                            <div className="flex items-center justify-end space-x-2 opacity-0 group-hover:opacity-100 transition-opacity">
                                                <button className="p-1 text-gray-400 hover:text-primary-600 dark:hover:text-primary-400">
                                                    <Download className="w-4 h-4" />
                                                </button>
                                                <button className="p-1 text-gray-400 hover:text-red-600 dark:hover:text-red-400">
                                                    <Trash2 className="w-4 h-4" />
                                                </button>
                                                <button className="p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300">
                                                    <MoreVertical className="w-4 h-4" />
                                                </button>
                                            </div>
                                        </td>
                                    </tr>
                                ))
                            )}
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    );
}
