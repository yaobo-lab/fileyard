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
            case 'folder': return <Folder className="w-4 h-4 text-amber-500" />;
            case 'image': return <Image className="w-4 h-4 text-violet-500" />;
            case 'video': return <Film className="w-4 h-4 text-rose-500" />;
            default: return <FileText className="w-4 h-4 text-sky-500" />;
        }
    };

    const filteredFiles = files.filter(file =>
        file.name.toLowerCase().includes(searchQuery.toLowerCase())
    );

    return (
        <div className="space-y-6">
            <div className="flex flex-col sm:flex-row justify-between items-start sm:items-center gap-4">
                <h1 className="text-xl sm:text-2xl font-bold text-foreground">Files</h1>

                <div className="flex items-center gap-3 w-full sm:w-auto">
                    {isAdmin && (
                        <select
                            value={selectedDepartment}
                            onChange={(e) => setSelectedDepartment(e.target.value)}
                            className="px-3 py-1.5 border border-border rounded-lg bg-card text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring"
                        >
                            <option value="">All Departments</option>
                            {departments.map(dept => (
                                <option key={dept.id} value={dept.id}>{dept.name}</option>
                            ))}
                        </select>
                    )}
                    <button className="px-3 py-1.5 bg-foreground text-background rounded-lg hover:bg-foreground/90 transition-colors text-sm font-medium">
                        Upload File
                    </button>
                </div>
            </div>

            {/* Breadcrumbs & Search */}
            <div className="flex flex-col sm:flex-row justify-between items-center gap-4 bg-card p-4 rounded-xl border border-border">
                <div className="flex items-center text-sm text-muted-foreground">
                    <button
                        onClick={() => setCurrentPath('/')}
                        className="hover:text-foreground flex items-center transition-colors"
                    >
                        <Home className="w-4 h-4 mr-1.5" />
                        Home
                    </button>
                    {currentPath !== '/' && currentPath.split('/').filter(Boolean).map((part, index, arr) => (
                        <div key={index} className="flex items-center">
                            <ChevronRight className="w-4 h-4 mx-1.5 text-muted-foreground/50" />
                            <span className="font-medium text-foreground">{part}</span>
                        </div>
                    ))}
                </div>

                <div className="relative w-full sm:w-64">
                    <Search className="absolute left-3 top-2.5 w-4 h-4 text-muted-foreground/60" />
                    <input
                        type="text"
                        placeholder="Search files..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        className="w-full pl-9 pr-4 py-1.5 border border-border rounded-lg bg-muted/30 text-sm text-foreground placeholder-muted-foreground/60 focus:outline-none focus:ring-2 focus:ring-ring"
                    />
                </div>
            </div>

            {/* File List */}
            <div className="bg-card rounded-xl border border-border overflow-hidden">
                <div className="overflow-x-auto">
                    <table className="min-w-full divide-y divide-border">
                        <thead className="bg-muted/50">
                            <tr>
                                <th className="px-5 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">Name</th>
                                <th className="px-5 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">Size</th>
                                <th className="px-5 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">Modified</th>
                                <th className="px-5 py-3 text-left text-xs font-semibold text-muted-foreground uppercase tracking-wider">Owner</th>
                                <th className="relative px-5 py-3"><span className="sr-only">Actions</span></th>
                            </tr>
                        </thead>
                        <tbody className="divide-y divide-border">
                            {isLoading ? (
                                <tr>
                                    <td colSpan={5} className="px-5 py-12 text-center text-muted-foreground">
                                        Loading files...
                                    </td>
                                </tr>
                            ) : filteredFiles.length === 0 ? (
                                <tr>
                                    <td colSpan={5} className="px-5 py-12 text-center text-muted-foreground">
                                        <div className="flex flex-col items-center justify-center">
                                            <Folder className="w-10 h-10 text-muted-foreground/40 mb-3" />
                                            <p className="text-sm font-medium text-foreground">No files found</p>
                                            <p className="text-xs text-muted-foreground mt-1">Upload a file to get started</p>
                                        </div>
                                    </td>
                                </tr>
                            ) : (
                                filteredFiles.map((file) => (
                                    <tr key={file.id} className="hover:bg-muted/40 transition-colors group">
                                        <td className="px-5 py-4 whitespace-nowrap">
                                            <div className="flex items-center">
                                                <div className="flex-shrink-0 h-8 w-8 flex items-center justify-center rounded-lg bg-muted">
                                                    {getFileIcon(file.type)}
                                                </div>
                                                <div className="ml-3">
                                                    <div className="text-sm font-medium text-foreground">{file.name}</div>
                                                </div>
                                            </div>
                                        </td>
                                        <td className="px-5 py-4 whitespace-nowrap text-sm text-muted-foreground">
                                            {file.size}
                                        </td>
                                        <td className="px-5 py-4 whitespace-nowrap text-sm text-muted-foreground">
                                            {formatDistanceToNow(new Date(file.modified), { addSuffix: true })}
                                        </td>
                                        <td className="px-5 py-4 whitespace-nowrap text-sm text-muted-foreground">
                                            {file.owner}
                                        </td>
                                        <td className="px-5 py-4 whitespace-nowrap text-right text-sm font-medium">
                                            <div className="flex items-center justify-end space-x-2 opacity-0 group-hover:opacity-100 transition-opacity">
                                                <button className="p-1 text-muted-foreground hover:text-foreground rounded transition-colors">
                                                    <Download className="w-4 h-4" />
                                                </button>
                                                <button className="p-1 text-muted-foreground hover:text-destructive rounded transition-colors">
                                                    <Trash2 className="w-4 h-4" />
                                                </button>
                                                <button className="p-1 text-muted-foreground hover:text-foreground rounded transition-colors">
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
