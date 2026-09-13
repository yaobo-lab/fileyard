import { useRef, useState, useEffect } from 'react';
import { Upload, Trash2, Loader2, Save, Check } from 'lucide-react';
import { useGlobalSettings } from '../../context/GlobalSettingsContext';
import { useTranslations } from '../../context/I18nContext';
import { useModalDialog } from '../../context/ModalDialogContext';
import { Logo } from '../../components/Logo';
import { ImageCropModal } from '../../components/ImageCropModal';
import clsx from 'clsx';

export function BrandingSettings() {
    const t = useTranslations('SettingsBranding');
    const tCommon = useTranslations('Common');
    const { alert: modalAlert, confirm: modalConfirm } = useModalDialog();
    const { settings, uploadLogo, deleteLogo, uploadFavicon, deleteFavicon, updateSettings } = useGlobalSettings();
    const logoInputRef = useRef<HTMLInputElement>(null);
    const faviconInputRef = useRef<HTMLInputElement>(null);
    const [isUploadingLogo, setIsUploadingLogo] = useState(false);
    const [isUploadingFavicon, setIsUploadingFavicon] = useState(false);

    // Crop modal state
    const [showCropModal, setShowCropModal] = useState(false);
    const [selectedImage, setSelectedImage] = useState<string | null>(null);
    const [cropTarget, setCropTarget] = useState<'logo' | 'favicon'>('logo');

    // Footer settings state
    const [footerAttribution, setFooterAttribution] = useState(settings.footer_attribution);
    const [footerDisclaimer, setFooterDisclaimer] = useState(settings.footer_disclaimer);
    const [isSaving, setIsSaving] = useState(false);
    const [saveSuccess, setSaveSuccess] = useState(false);

    const hasChanges =
        footerAttribution !== settings.footer_attribution ||
        footerDisclaimer !== settings.footer_disclaimer;

    useEffect(() => {
        setFooterAttribution(settings.footer_attribution);
        setFooterDisclaimer(settings.footer_disclaimer);
    }, [settings]);

    const handleLogoFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (!file) return;

        // For SVG files, skip cropping and upload directly
        if (file.type === 'image/svg+xml') {
            setIsUploadingLogo(true);
            await uploadLogo(file);
            setIsUploadingLogo(false);
            if (logoInputRef.current) {
                logoInputRef.current.value = '';
            }
            return;
        }

        const validTypes = ['image/png', 'image/jpeg', 'image/webp', 'image/gif'];
        if (!validTypes.includes(file.type)) {
            modalAlert({
                title: tCommon('errorTitle') || 'Error',
                description: 'Please upload an SVG, PNG, JPEG, WebP, or GIF file',
                variant: 'destructive'
            });
            return;
        }

        if (file.size > 2 * 1024 * 1024) {
            modalAlert({
                title: tCommon('errorTitle') || 'Error',
                description: 'Logo must be less than 2MB',
                variant: 'destructive'
            });
            return;
        }

        // Convert file to data URL for the cropper
        const reader = new FileReader();
        reader.onload = () => {
            setSelectedImage(reader.result as string);
            setCropTarget('logo');
            setShowCropModal(true);
        };
        reader.readAsDataURL(file);

        // Reset the input
        if (logoInputRef.current) {
            logoInputRef.current.value = '';
        }
    };

    const handleFaviconFileSelect = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const file = e.target.files?.[0];
        if (!file) return;

        // For SVG and ICO files, skip cropping and upload directly
        if (file.type === 'image/svg+xml' || file.type === 'image/x-icon' || file.type === 'image/vnd.microsoft.icon') {
            setIsUploadingFavicon(true);
            await uploadFavicon(file);
            setIsUploadingFavicon(false);
            if (faviconInputRef.current) {
                faviconInputRef.current.value = '';
            }
            return;
        }

        const validTypes = ['image/png', 'image/jpeg', 'image/webp', 'image/gif'];
        if (!validTypes.includes(file.type)) {
            modalAlert({
                title: tCommon('errorTitle') || 'Error',
                description: 'Please upload an ICO, SVG, PNG, JPEG, WebP, or GIF file',
                variant: 'destructive'
            });
            return;
        }

        if (file.size > 1024 * 1024) {
            modalAlert({
                title: tCommon('errorTitle') || 'Error',
                description: 'Favicon must be less than 1MB',
                variant: 'destructive'
            });
            return;
        }

        // Convert file to data URL for the cropper
        const reader = new FileReader();
        reader.onload = () => {
            setSelectedImage(reader.result as string);
            setCropTarget('favicon');
            setShowCropModal(true);
        };
        reader.readAsDataURL(file);

        // Reset the input
        if (faviconInputRef.current) {
            faviconInputRef.current.value = '';
        }
    };

    const handleCropComplete = async (croppedBlob: Blob) => {
        setShowCropModal(false);
        setSelectedImage(null);

        if (cropTarget === 'logo') {
            setIsUploadingLogo(true);
            const file = new File([croppedBlob], 'logo.png', { type: 'image/png' });
            await uploadLogo(file);
            setIsUploadingLogo(false);
        } else {
            setIsUploadingFavicon(true);
            const file = new File([croppedBlob], 'favicon.png', { type: 'image/png' });
            await uploadFavicon(file);
            setIsUploadingFavicon(false);
        }
    };

    const handleCropCancel = () => {
        setShowCropModal(false);
        setSelectedImage(null);
    };

    const handleLogoDelete = async () => {
        const confirmed = await modalConfirm({
            title: tCommon('deleteConfirmTitle'),
            description: 'Are you sure you want to remove the custom logo?',
            variant: 'destructive',
            confirmText: tCommon('delete'),
            cancelText: tCommon('cancel')
        });
        if (!confirmed) return;
        await deleteLogo();
    };

    const handleFaviconDelete = async () => {
        const confirmed = await modalConfirm({
            title: tCommon('deleteConfirmTitle'),
            description: 'Are you sure you want to remove the custom favicon?',
            variant: 'destructive',
            confirmText: tCommon('delete'),
            cancelText: tCommon('cancel')
        });
        if (!confirmed) return;
        await deleteFavicon();
    };

    const handleSaveFooter = async () => {
        setIsSaving(true);
        setSaveSuccess(false);

        const success = await updateSettings({
            footer_attribution: footerAttribution,
            footer_disclaimer: footerDisclaimer,
        });

        setIsSaving(false);
        if (success) {
            setSaveSuccess(true);
            setTimeout(() => setSaveSuccess(false), 3000);
        }
    };

    return (
        <div className="space-y-6">
            <div className="flex items-center justify-between">
                <div>
                    <h2 className="text-lg font-semibold text-gray-900 dark:text-white">{t('title')}</h2>
                    <p className="text-sm text-gray-500 dark:text-gray-400">{t('description')}</p>
                </div>
                <button
                    onClick={handleSaveFooter}
                    disabled={!hasChanges || isSaving}
                    className={clsx(
                        "flex items-center px-4 py-2 rounded-lg text-sm font-medium transition-all",
                        hasChanges && !isSaving
                            ? "bg-primary-600 text-white hover:bg-primary-700"
                            : "bg-gray-100 dark:bg-gray-700 text-gray-400 cursor-not-allowed"
                    )}
                >
                    {isSaving ? (
                        <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                    ) : saveSuccess ? (
                        <Check className="w-4 h-4 mr-2" />
                    ) : (
                        <Save className="w-4 h-4 mr-2" />
                    )}
                    {isSaving ? tCommon('saving') : saveSuccess ? tCommon('saved') : tCommon('save')}
                </button>
            </div>

            {/* Logo Upload */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="font-medium text-gray-900 dark:text-white">{t('appLogo')}</h3>
                </div>
                <div className="p-6">
                    <div className="flex items-start gap-8">
                        {/* Logo Preview */}
                        <div className="flex-shrink-0">
                            <div className="w-32 h-32 bg-gray-100 dark:bg-gray-700 rounded-2xl border-2 border-dashed border-gray-300 dark:border-gray-600 flex items-center justify-center overflow-hidden">
                                {settings.logo_url ? (
                                    <img
                                        src={settings.logo_url}
                                        alt="Custom Logo"
                                        className="max-w-full max-h-full object-contain p-3"
                                    />
                                ) : (
                                    <Logo className="h-20 w-auto text-gray-400 dark:text-gray-500" forceDefault />
                                )}
                            </div>
                            <p className="mt-2 text-xs text-center text-gray-500 dark:text-gray-400">
                                {settings.logo_url ? tCommon('edit') : 'Default'}
                            </p>
                        </div>

                        {/* Upload Controls */}
                        <div className="flex-1">
                            <input
                                ref={logoInputRef}
                                type="file"
                                accept="image/svg+xml,image/png,image/jpeg,image/webp,image/gif"
                                onChange={handleLogoFileSelect}
                                className="hidden"
                            />
                            <div className="space-y-4">
                                <div className="flex gap-3">
                                    <button
                                        onClick={() => logoInputRef.current?.click()}
                                        disabled={isUploadingLogo}
                                        className="flex items-center px-4 py-2.5 bg-primary-600 text-white rounded-lg hover:bg-primary-700 text-sm font-medium disabled:opacity-50 transition-colors"
                                    >
                                        {isUploadingLogo ? (
                                            <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                                        ) : (
                                            <Upload className="w-4 h-4 mr-2" />
                                        )}
                                        {t('uploadLogo')}
                                    </button>

                                    {settings.logo_url && (
                                        <button
                                            onClick={handleLogoDelete}
                                            className="flex items-center px-4 py-2.5 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm font-medium transition-colors"
                                        >
                                            <Trash2 className="w-4 h-4 mr-2" />
                                            {t('removeLogo')}
                                        </button>
                                    )}
                                </div>

                                <p className="text-xs text-gray-500 dark:text-gray-400">
                                    {t('logoFormatHint')}
                                </p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            {/* Favicon Upload */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="font-medium text-gray-900 dark:text-white">{t('browserFavicon')}</h3>
                </div>
                <div className="p-6">
                    <div className="flex items-start gap-8">
                        {/* Favicon Preview */}
                        <div className="flex-shrink-0">
                            <div className="w-24 h-24 bg-gray-100 dark:bg-gray-700 rounded-xl border-2 border-dashed border-gray-300 dark:border-gray-600 flex items-center justify-center overflow-hidden">
                                {settings.favicon_url ? (
                                    <img
                                        src={settings.favicon_url}
                                        alt="Custom Favicon"
                                        className="w-12 h-12 object-contain"
                                    />
                                ) : (
                                    <svg className="w-12 h-12 text-gray-400 dark:text-gray-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5">
                                        <path d="M12 2L2 7l10 5 10-5-10-5z" />
                                        <path d="M2 17l10 5 10-5" />
                                        <path d="M2 12l10 5 10-5" />
                                    </svg>
                                )}
                            </div>
                            <p className="mt-2 text-xs text-center text-gray-500 dark:text-gray-400">
                                {settings.favicon_url ? tCommon('edit') : 'Default'}
                            </p>
                        </div>

                        {/* Upload Controls */}
                        <div className="flex-1">
                            <input
                                ref={faviconInputRef}
                                type="file"
                                accept="image/x-icon,image/vnd.microsoft.icon,image/svg+xml,image/png,image/jpeg,image/webp,image/gif"
                                onChange={handleFaviconFileSelect}
                                className="hidden"
                            />
                            <div className="space-y-4">
                                <div className="flex gap-3">
                                    <button
                                        onClick={() => faviconInputRef.current?.click()}
                                        disabled={isUploadingFavicon}
                                        className="flex items-center px-4 py-2.5 bg-primary-600 text-white rounded-lg hover:bg-primary-700 text-sm font-medium disabled:opacity-50 transition-colors"
                                    >
                                        {isUploadingFavicon ? (
                                            <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                                        ) : (
                                            <Upload className="w-4 h-4 mr-2" />
                                        )}
                                        {t('uploadFavicon')}
                                    </button>

                                    {settings.favicon_url && (
                                        <button
                                            onClick={handleFaviconDelete}
                                            className="flex items-center px-4 py-2.5 border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-50 dark:hover:bg-gray-700 rounded-lg text-sm font-medium transition-colors"
                                        >
                                            <Trash2 className="w-4 h-4 mr-2" />
                                            {t('removeFavicon')}
                                        </button>
                                    )}
                                </div>

                                <p className="text-xs text-gray-500 dark:text-gray-400">
                                    {t('faviconFormatHint')}
                                </p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            {/* Footer Content */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="font-medium text-gray-900 dark:text-white">{t('footerSettings')}</h3>
                </div>
                <div className="p-6 space-y-6">
                    <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                            {t('footerAttribution')}
                        </label>
                        <input
                            type="text"
                            value={footerAttribution}
                            onChange={(e) => setFooterAttribution(e.target.value)}
                            placeholder="An open source project by Fileyard.org"
                            className="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
                        />
                        <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">
                            {t('footerAttributionDesc')}
                        </p>
                    </div>

                    <div>
                        <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                            {t('footerDisclaimer')}
                        </label>
                        <textarea
                            value={footerDisclaimer}
                            onChange={(e) => setFooterDisclaimer(e.target.value)}
                            rows={3}
                            placeholder="Enter your legal disclaimer..."
                            className="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
                        />
                        <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">
                            {t('footerDisclaimerDesc')}
                        </p>
                    </div>
                </div>
            </div>

            {/* Crop Modal for Logo/Favicon */}
            {showCropModal && selectedImage && (
                <ImageCropModal
                    image={selectedImage}
                    onCropComplete={handleCropComplete}
                    onCancel={handleCropCancel}
                    cropShape="rect"
                    aspect={1}
                    title={cropTarget === 'logo' ? 'Crop Logo' : 'Crop Favicon'}
                />
            )}
        </div>
    );
}
