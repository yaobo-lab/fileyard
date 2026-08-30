import { useState, useEffect } from 'react';
import { PieChart, Pie, Cell, ResponsiveContainer, Tooltip, Legend } from 'recharts';
import { FileText } from 'lucide-react';
import { useAuthFetch } from '../../context/AuthContext';

interface FileTypeData {
    name: string;
    value: number;
    color: string;
    [key: string]: string | number;
}

const COLORS = [
    '#3B82F6', // blue
    '#10B981', // green
    '#F59E0B', // amber
    '#EF4444', // red
    '#8B5CF6', // purple
    '#EC4899', // pink
    '#06B6D4', // cyan
    '#84CC16', // lime
];

const FILE_TYPE_LABELS: Record<string, string> = {
    'application/pdf': 'PDF',
    'image/jpeg': 'Images',
    'image/png': 'Images',
    'image/gif': 'Images',
    'image/webp': 'Images',
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document': 'Documents',
    'application/msword': 'Documents',
    'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet': 'Spreadsheets',
    'application/vnd.ms-excel': 'Spreadsheets',
    'text/plain': 'Text Files',
    'text/csv': 'CSV',
    'application/json': 'JSON',
    'application/zip': 'Archives',
    'application/x-rar-compressed': 'Archives',
    'video/mp4': 'Videos',
    'video/quicktime': 'Videos',
    'audio/mpeg': 'Audio',
    'audio/wav': 'Audio',
};

export function FileTypesChartWidget() {
    const [fileTypes, setFileTypes] = useState<FileTypeData[]>([]);
    const [isLoading, setIsLoading] = useState(true);
    const [totalFiles, setTotalFiles] = useState(0);
    const authFetch = useAuthFetch();

    useEffect(() => {
        fetchFileTypes();
    }, []);

    const fetchFileTypes = async () => {
        try {
            const res = await authFetch('/api/dashboard/file-types');
            if (res.ok) {
                const data = await res.json();
                const typeData: FileTypeData[] = (data.file_types || []).map((item: any, index: number) => ({
                    name: item.label || getTypeLabel(item.content_type),
                    value: item.count,
                    color: COLORS[index % COLORS.length]
                }));
                setFileTypes(typeData);
                setTotalFiles(data.total || typeData.reduce((sum: number, t: FileTypeData) => sum + t.value, 0));
            }
        } catch (error) {
            console.error('Failed to fetch file types', error);
        } finally {
            setIsLoading(false);
        }
    };

    const getTypeLabel = (contentType: string): string => {
        if (FILE_TYPE_LABELS[contentType]) {
            return FILE_TYPE_LABELS[contentType];
        }
        if (contentType.startsWith('image/')) return 'Images';
        if (contentType.startsWith('video/')) return 'Videos';
        if (contentType.startsWith('audio/')) return 'Audio';
        if (contentType.startsWith('text/')) return 'Text Files';
        return 'Other';
    };

    const CustomTooltip = ({ active, payload }: any) => {
        if (active && payload && payload.length) {
            const data = payload[0].payload;
            const percentage = totalFiles > 0 ? ((data.value / totalFiles) * 100).toFixed(1) : 0;
            return (
                <div className="bg-gray-900 dark:bg-gray-700 text-white px-3 py-2 rounded-lg shadow-lg text-sm">
                    <p className="font-medium">{data.name}</p>
                    <p className="text-gray-300">{data.value} files ({percentage}%)</p>
                </div>
            );
        }
        return null;
    };

    const renderCustomizedLabel = ({ cx, cy, midAngle, innerRadius, outerRadius, percent }: any) => {
        if (percent < 0.05) return null; // Don't show labels for very small slices
        const RADIAN = Math.PI / 180;
        const radius = innerRadius + (outerRadius - innerRadius) * 0.5;
        const x = cx + radius * Math.cos(-midAngle * RADIAN);
        const y = cy + radius * Math.sin(-midAngle * RADIAN);

        return (
            <text 
                x={x} 
                y={y} 
                fill="white" 
                textAnchor="middle" 
                dominantBaseline="central"
                className="text-xs font-medium"
            >
                {`${(percent * 100).toFixed(0)}%`}
            </text>
        );
    };

    return (
        <div className="bg-white dark:bg-gray-800 rounded-lg shadow-sm border border-gray-200 dark:border-gray-700 p-6 h-full">
            <div className="flex items-center justify-between mb-4">
                <div>
                    <h3 className="text-lg font-medium text-gray-900 dark:text-white">File Types</h3>
                    <p className="text-sm text-gray-500 dark:text-gray-400">Distribution by type</p>
                </div>
                <div className="flex items-center space-x-2">
                    <div className="p-2 bg-green-100 dark:bg-green-900/30 rounded-lg">
                        <FileText className="w-5 h-5 text-green-600 dark:text-green-400" />
                    </div>
                    <div className="text-right">
                        <p className="text-2xl font-bold text-gray-900 dark:text-white">{totalFiles}</p>
                        <p className="text-xs text-gray-500 dark:text-gray-400">Total</p>
                    </div>
                </div>
            </div>
            
            {isLoading ? (
                <div className="h-48 flex items-center justify-center">
                    <div className="animate-pulse w-32 h-32 bg-gray-200 dark:bg-gray-700 rounded-full"></div>
                </div>
            ) : fileTypes.length === 0 ? (
                <div className="h-48 flex items-center justify-center text-gray-500 dark:text-gray-400">
                    <div className="text-center">
                        <FileText className="w-8 h-8 mx-auto mb-2 opacity-50" />
                        <p className="text-sm">No files uploaded yet</p>
                    </div>
                </div>
            ) : (
                <div className="h-[200px] w-full">
                    <ResponsiveContainer width="100%" height={200}>
                        <PieChart>
                            <Pie
                                data={fileTypes}
                                cx="50%"
                                cy="50%"
                                labelLine={false}
                                label={renderCustomizedLabel}
                                innerRadius={40}
                                outerRadius={70}
                                paddingAngle={2}
                                dataKey="value"
                            >
                                {fileTypes.map((entry, index) => (
                                    <Cell 
                                        key={`cell-${index}`} 
                                        fill={entry.color}
                                        className="transition-all duration-300 hover:opacity-80"
                                        stroke="transparent"
                                    />
                                ))}
                            </Pie>
                            <Tooltip content={<CustomTooltip />} />
                        </PieChart>
                    </ResponsiveContainer>
                </div>
            )}
            
            {/* Legend */}
            {fileTypes.length > 0 && (
                <div className="mt-4 flex flex-wrap gap-3 justify-center">
                    {fileTypes.slice(0, 4).map((type, index) => (
                        <div key={index} className="flex items-center space-x-1.5">
                            <div 
                                className="w-2.5 h-2.5 rounded-full" 
                                style={{ backgroundColor: type.color }}
                            />
                            <span className="text-xs text-gray-600 dark:text-gray-400">{type.name}</span>
                        </div>
                    ))}
                    {fileTypes.length > 4 && (
                        <span className="text-xs text-gray-500 dark:text-gray-400">
                            +{fileTypes.length - 4} more
                        </span>
                    )}
                </div>
            )}
        </div>
    );
}
