import { BackupRestore } from '../../components/BackupRestore';
import { useTranslations } from '../../context/I18nContext';

export function BackupRestoreSettings() {
    const t = useTranslations('SettingsBackup');

    return (
        <div className="space-y-6">
            <div>
                <h1 className="text-2xl font-bold text-gray-900 dark:text-white">{t('title')}</h1>
                <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">
                    {t('description')}
                </p>
            </div>
            <BackupRestore type="global" />
        </div>
    );
}
