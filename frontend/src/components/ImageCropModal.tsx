import { useState, useCallback } from 'react';
import Cropper, { Area, MediaSize } from 'react-easy-crop';
import { X, ZoomIn, ZoomOut, Check } from 'lucide-react';
import clsx from 'clsx';

interface ImageCropModalProps {
    image: string;
    onCropComplete: (croppedBlob: Blob) => void;
    onCancel: () => void;
    /** Crop shape: 'round' for avatars, 'rect' for logos. Default: 'round' */
    cropShape?: 'round' | 'rect';
    /** Aspect ratio for the crop area. Default: 1 (square) */
    aspect?: number;
    /** Modal title. Default: 'Crop Avatar' */
    title?: string;
}

// Helper function to create cropped image
async function getCroppedImg(imageSrc: string, pixelCrop: Area): Promise<Blob> {
    const image = new Image();
    
    // Set onload/onerror BEFORE setting src to avoid race condition
    // where image loads before handlers are attached
    await new Promise<void>((resolve, reject) => {
        image.onload = () => resolve();
        image.onerror = () => reject(new Error('Failed to load image'));
        image.src = imageSrc;
    });

    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    
    if (!ctx) {
        throw new Error('No 2d context');
    }

    // Set canvas size to the cropped area
    canvas.width = pixelCrop.width;
    canvas.height = pixelCrop.height;

    // Draw the cropped image
    ctx.drawImage(
        image,
        pixelCrop.x,
        pixelCrop.y,
        pixelCrop.width,
        pixelCrop.height,
        0,
        0,
        pixelCrop.width,
        pixelCrop.height
    );

    // Return as blob
    return new Promise((resolve, reject) => {
        canvas.toBlob(
            (blob) => {
                if (blob) {
                    resolve(blob);
                } else {
                    reject(new Error('Canvas is empty'));
                }
            },
            'image/png',
            1
        );
    });
}

export function ImageCropModal({ 
    image, 
    onCropComplete, 
    onCancel,
    cropShape = 'round',
    aspect = 1,
    title = 'Crop Avatar'
}: ImageCropModalProps) {
    const [crop, setCrop] = useState({ x: 0, y: 0 });
    const [zoom, setZoom] = useState(1);
    const [croppedAreaPixels, setCroppedAreaPixels] = useState<Area | null>(null);
    const [isProcessing, setIsProcessing] = useState(false);
    const [isMediaLoaded, setIsMediaLoaded] = useState(false);

    const onCropChange = useCallback((crop: { x: number; y: number }) => {
        setCrop(crop);
    }, []);

    const onZoomChange = useCallback((zoom: number) => {
        setZoom(zoom);
    }, []);

    const onCropCompleteCallback = useCallback((_: Area, croppedAreaPixels: Area) => {
        console.log('Crop complete callback:', croppedAreaPixels);
        setCroppedAreaPixels(croppedAreaPixels);
    }, []);

    // Set initial crop area when media loads
    const onMediaLoaded = useCallback((mediaSize: MediaSize) => {
        console.log('Media loaded:', mediaSize);
        setIsMediaLoaded(true);
        
        // Calculate initial centered crop based on aspect ratio
        const mediaAspect = mediaSize.width / mediaSize.height;
        let cropWidth: number;
        let cropHeight: number;
        
        if (mediaAspect > aspect) {
            // Image is wider than crop aspect
            cropHeight = mediaSize.height;
            cropWidth = cropHeight * aspect;
        } else {
            // Image is taller than crop aspect
            cropWidth = mediaSize.width;
            cropHeight = cropWidth / aspect;
        }
        
        const initialCrop: Area = {
            x: (mediaSize.width - cropWidth) / 2,
            y: (mediaSize.height - cropHeight) / 2,
            width: cropWidth,
            height: cropHeight,
        };
        
        console.log('Setting initial crop:', initialCrop);
        setCroppedAreaPixels(initialCrop);
    }, [aspect]);

    const handleConfirm = async () => {
        console.log('Handle confirm called, croppedAreaPixels:', croppedAreaPixels);
        
        if (!croppedAreaPixels) {
            alert('Please wait for the image to load or adjust the crop area');
            return;
        }
        
        setIsProcessing(true);
        try {
            console.log('Getting cropped image...');
            const croppedBlob = await getCroppedImg(image, croppedAreaPixels);
            console.log('Got cropped blob:', croppedBlob.size, 'bytes');
            onCropComplete(croppedBlob);
        } catch (error) {
            console.error('Error cropping image:', error);
            alert(`Error processing image: ${error instanceof Error ? error.message : 'Unknown error'}`);
        } finally {
            setIsProcessing(false);
        }
    };

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70">
            <div className="bg-white dark:bg-gray-800 rounded-xl shadow-2xl w-full max-w-lg mx-4 overflow-hidden">
                {/* Header */}
                <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                        {title}
                    </h3>
                    <button
                        onClick={onCancel}
                        className="p-2 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <X className="w-5 h-5" />
                    </button>
                </div>

                {/* Cropper Area */}
                <div className="relative h-80 bg-gray-900">
                    <Cropper
                        image={image}
                        crop={crop}
                        zoom={zoom}
                        aspect={aspect}
                        cropShape={cropShape}
                        showGrid={cropShape === 'rect'}
                        onCropChange={onCropChange}
                        onZoomChange={onZoomChange}
                        onCropComplete={onCropCompleteCallback}
                        onMediaLoaded={onMediaLoaded}
                    />
                </div>

                {/* Zoom Controls */}
                <div className="p-4 border-t border-gray-200 dark:border-gray-700">
                    <div className="flex items-center gap-4">
                        <ZoomOut className="w-5 h-5 text-gray-500 dark:text-gray-400" />
                        <input
                            type="range"
                            min={1}
                            max={3}
                            step={0.1}
                            value={zoom}
                            onChange={(e) => setZoom(Number(e.target.value))}
                            className="flex-1 h-2 bg-gray-200 dark:bg-gray-700 rounded-lg appearance-none cursor-pointer accent-blue-600"
                        />
                        <ZoomIn className="w-5 h-5 text-gray-500 dark:text-gray-400" />
                    </div>
                </div>

                {/* Actions */}
                <div className="flex gap-3 p-4 border-t border-gray-200 dark:border-gray-700">
                    <button
                        onClick={onCancel}
                        className="flex-1 px-4 py-2.5 text-gray-700 dark:text-gray-300 bg-gray-100 dark:bg-gray-700 hover:bg-gray-200 dark:hover:bg-gray-600 rounded-lg font-medium transition-colors"
                    >
                        Cancel
                    </button>
                    <button
                        onClick={handleConfirm}
                        disabled={isProcessing || !croppedAreaPixels}
                        className={clsx(
                            "flex-1 px-4 py-2.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg font-medium transition-colors flex items-center justify-center gap-2",
                            (isProcessing || !croppedAreaPixels) && "opacity-70 cursor-not-allowed"
                        )}
                    >
                        {isProcessing ? (
                            <>
                                <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                                Processing...
                            </>
                        ) : !croppedAreaPixels ? (
                            <>
                                <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                                Loading...
                            </>
                        ) : (
                            <>
                                <Check className="w-4 h-4" />
                                Apply
                            </>
                        )}
                    </button>
                </div>
            </div>
        </div>
    );
}
