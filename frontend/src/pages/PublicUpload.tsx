import React, { useState, useCallback } from 'react';
import { useParams } from 'react-router-dom';
import { UploadCloud, CheckCircle, AlertCircle, File, Shield } from 'lucide-react';
import clsx from 'clsx';
import { Logo } from '../components/Logo';

export function PublicUpload() {
    const { token } = useParams<{ token: string }>();
    const [isDragging, setIsDragging] = useState(false);
    const [uploadStatus, setUploadStatus] = useState<'idle' | 'uploading' | 'success' | 'error'>('idle');
    const [fileName, setFileName] = useState<string | null>(null);
    const [errorMessage, setErrorMessage] = useState<string | null>(null);

    const handleDrag = (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        if (e.type === "dragenter" || e.type === "dragover") {
            setIsDragging(true);
        } else if (e.type === "dragleave") {
            setIsDragging(false);
        }
    };

    const handleDrop = useCallback(async (e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setIsDragging(false);

        const droppedFiles = Array.from(e.dataTransfer.files);
        if (droppedFiles.length === 0) return;

        uploadFile(droppedFiles[0]);
    }, [token]);

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        if (e.target.files && e.target.files[0]) {
            uploadFile(e.target.files[0]);
        }
    };

    const uploadFile = async (file: File) => {
        setFileName(file.name);
        setUploadStatus('uploading');
        setErrorMessage(null);

        try {
            const formData = new FormData();
            formData.append('file', file);

            const API_URL = import.meta.env.VITE_API_URL || '';
            const response = await fetch(`${API_URL}/api/public-upload/${token}`, {
                method: 'POST',
                body: formData,
            });

            if (response.ok) {
                const data = await response.json();
                // Check for blocked extension error (returned as 200 with error field)
                if (data.error === 'blocked_extension') {
                    setErrorMessage(data.message || `File type .${data.extension} is not allowed`);
                    setUploadStatus('error');
                } else {
                    setUploadStatus('success');
                }
            } else {
                setUploadStatus('error');
                setErrorMessage('Upload failed. Please try again.');
            }
        } catch (error) {
            console.error('Upload error:', error);
            setUploadStatus('error');
            setErrorMessage('Upload failed. Please check your connection and try again.');
        }
    };

    return (
        <div className="min-h-screen bg-white flex flex-col items-center justify-center p-4 relative overflow-hidden">
            {/* Background decoration */}
            <div className="absolute top-0 left-0 w-full h-full overflow-hidden z-0 pointer-events-none">
                <div className="absolute top-1/4 left-1/4 w-96 h-96 bg-primary-600/20 rounded-full blur-3xl"></div>
                <div className="absolute bottom-1/4 right-1/4 w-96 h-96 bg-blue-600/20 rounded-full blur-3xl"></div>
            </div>

            <div className="w-full max-w-md z-10">
                <div className="text-center mb-8">
                    <div className="flex justify-center mb-4">
                        <div className="h-60 w-auto text-primary-600">
                            <Logo className="h-60 w-auto text-primary-600" />
                        </div>
                    </div>
                    <h1 className="text-3xl font-bold text-gray-900 tracking-tight">Secure File Upload</h1>
                    <p className="text-gray-500 mt-2">You have been invited to securely upload documents.</p>
                </div>

                <div className="bg-white border border-gray-200 rounded-2xl shadow-xl ring-1 ring-gray-900/5 overflow-hidden">
                    <div className="p-8">
                        {uploadStatus === 'success' ? (
                            <div className="text-center py-8">
                                <div className="mx-auto flex items-center justify-center h-20 w-20 rounded-full bg-gradient-to-br from-green-400 to-green-600 mb-6 animate-pulse">
                                    <CheckCircle className="h-10 w-10 text-white" />
                                </div>
                                <h3 className="text-2xl font-bold text-gray-900 mb-2">Upload Complete!</h3>
                                <p className="text-gray-500 mb-2">
                                    <span className="font-semibold text-gray-900">{fileName}</span> has been securely uploaded.
                                </p>
                                <div className="inline-flex items-center gap-2 px-4 py-2 rounded-lg bg-gray-50 border border-gray-200 text-sm text-gray-500 mb-8">
                                    <Shield className="w-4 h-4 text-green-500" />
                                    <span>End-to-end encrypted and secured</span>
                                </div>
                                <button
                                    onClick={() => {
                                        setUploadStatus('idle');
                                        setFileName(null);
                                    }}
                                    className="w-full py-3 px-4 bg-gradient-to-r from-primary-600 to-primary-700 text-white rounded-xl font-semibold hover:from-primary-700 hover:to-primary-800 transition-all duration-200 shadow-lg shadow-primary-500/50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-offset-gray-900 focus:ring-primary-500"
                                >
                                    Upload Another File
                                </button>
                            </div>
                        ) : (
                            <>
                                <div
                                    onDragEnter={handleDrag}
                                    onDragLeave={handleDrag}
                                    onDragOver={handleDrag}
                                    onDrop={handleDrop}
                                    className={clsx(
                                        "relative group cursor-pointer flex flex-col items-center justify-center w-full h-64 rounded-xl border-2 border-dashed transition-all duration-200",
                                        isDragging
                                            ? "border-primary-400 bg-primary-50"
                                            : "border-gray-300 hover:border-gray-400 hover:bg-gray-50",
                                        uploadStatus === 'uploading' && "opacity-50 pointer-events-none"
                                    )}
                                >
                                    <input
                                        type="file"
                                        className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                                        onChange={handleChange}
                                        disabled={uploadStatus === 'uploading'}
                                    />

                                    <div className="flex flex-col items-center justify-center pt-5 pb-6 text-center">
                                        <div className={clsx(
                                            "p-4 rounded-full mb-4 transition-transform group-hover:scale-110 duration-200",
                                            isDragging ? "bg-primary-100" : "bg-gray-100"
                                        )}>
                                            <UploadCloud className={clsx(
                                                "w-10 h-10",
                                                isDragging ? "text-primary-600" : "text-gray-400"
                                            )} />
                                        </div>
                                        <p className="mb-2 text-lg font-medium text-gray-900">
                                            Drop files here or click to upload
                                        </p>
                                        <p className="text-sm text-gray-500">
                                            End-to-end encrypted transfer
                                        </p>
                                    </div>
                                </div>

                                {uploadStatus === 'uploading' && (
                                    <div className="mt-6">
                                        <div className="flex items-center justify-between text-sm text-gray-600 mb-2">
                                            <div className="flex items-center">
                                                <File className="w-4 h-4 mr-2" />
                                                <span className="truncate max-w-[200px]">{fileName}</span>
                                            </div>
                                            <span>Uploading...</span>
                                        </div>
                                        <div className="w-full bg-gray-200 rounded-full h-2">
                                            <div className="bg-primary-600 h-2 rounded-full animate-pulse w-full"></div>
                                        </div>
                                    </div>
                                )}

                                {uploadStatus === 'error' && (
                                    <div className="mt-6 p-4 bg-red-500/10 border border-red-500/20 rounded-xl flex items-start">
                                        <AlertCircle className="w-5 h-5 text-red-400 mt-0.5 mr-3 flex-shrink-0" />
                                        <div>
                                            <h4 className="text-sm font-medium text-red-400">Upload Failed</h4>
                                            <p className="text-sm text-red-300/80 mt-1">
                                                {errorMessage || 'Please check your connection and try again.'}
                                            </p>
                                        </div>
                                    </div>
                                )}
                            </>
                        )}
                    </div>

                    <div className="bg-gray-50 px-8 py-4 border-t border-gray-200 flex items-center justify-center text-xs text-gray-500">
                        <Shield className="w-3 h-3 mr-1.5" />
                        <span>256-bit SSL Secure Transfer</span>
                    </div>
                </div>

                <p className="mt-8 text-center text-xs text-gray-500">
                    &copy; {new Date().getFullYear()} ClovaLink. All rights reserved.
                </p>
            </div>
        </div>
    );
}
