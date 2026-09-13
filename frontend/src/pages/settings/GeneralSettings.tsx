import { useState, useEffect } from 'react';
import { Save, Check, Loader2, Globe } from 'lucide-react';
import { useGlobalSettings } from '../../context/GlobalSettingsContext';
import { useI18n, useTranslations } from '../../context/I18nContext';
import clsx from 'clsx';

const LANGUAGES = [
    {
        value: 'zh',
        label: '简体中文',
        nativeName: 'Simplified Chinese',
        code: 'zh-CN',
        descZh: '选择界面显示语言为中文。',
        descEn: 'Choose the interface language as Chinese.',
    },
    {
        value: 'en',
        label: 'English',
        nativeName: 'English (US)',
        code: 'en-US',
        descZh: '选择界面显示语言为英文。',
        descEn: 'Choose the interface language as English.',
    },
];

const DATE_FORMATS = [
    { value: 'MM/DD/YYYY', label: 'MM/DD/YYYY', description: 'United States' },
    { value: 'DD/MM/YYYY', label: 'DD/MM/YYYY', description: 'Europe / International' },
    { value: 'YYYY-MM-DD', label: 'YYYY-MM-DD', description: 'ISO 8601' },
];

const TIME_FORMATS = [
    { value: '12h', label: '12-hour', description: 'e.g., 2:30 PM' },
    { value: '24h', label: '24-hour', description: 'e.g., 14:30' },
];

const TIMEZONES = [
    { value: 'America/New_York', label: 'Eastern Time (ET)', offset: 'UTC-5' },
    { value: 'America/Chicago', label: 'Central Time (CT)', offset: 'UTC-6' },
    { value: 'America/Denver', label: 'Mountain Time (MT)', offset: 'UTC-7' },
    { value: 'America/Los_Angeles', label: 'Pacific Time (PT)', offset: 'UTC-8' },
    { value: 'America/Anchorage', label: 'Alaska Time (AKT)', offset: 'UTC-9' },
    { value: 'Pacific/Honolulu', label: 'Hawaii Time (HT)', offset: 'UTC-10' },
    { value: 'Europe/London', label: 'Greenwich Mean Time (GMT)', offset: 'UTC+0' },
    { value: 'Europe/Paris', label: 'Central European Time (CET)', offset: 'UTC+1' },
    { value: 'Asia/Tokyo', label: 'Japan Standard Time (JST)', offset: 'UTC+9' },
    { value: 'Asia/Shanghai', label: 'China Standard Time (CST)', offset: 'UTC+8' },
    { value: 'Australia/Sydney', label: 'Australian Eastern Time (AET)', offset: 'UTC+11' },
    { value: 'UTC', label: 'Coordinated Universal Time (UTC)', offset: 'UTC+0' },
];

export function GeneralSettings() {
    const { settings, updateSettings } = useGlobalSettings();
    const { locale, setLocale } = useI18n();
    const tSettings = useTranslations('Settings');
    const tCommon = useTranslations('Common');

    const [appName, setAppName] = useState(settings.app_name);
    const [dateFormat, setDateFormat] = useState(settings.date_format);
    const [timeFormat, setTimeFormat] = useState(settings.time_format);
    const [timezone, setTimezone] = useState(settings.timezone);
    const [language, setLanguage] = useState<'zh' | 'en'>(locale || settings.language || 'zh');
    const [isSaving, setIsSaving] = useState(false);
    const [saveSuccess, setSaveSuccess] = useState(false);

    const hasChanges =
        appName !== settings.app_name ||
        dateFormat !== settings.date_format ||
        timeFormat !== settings.time_format ||
        timezone !== settings.timezone ||
        language !== (settings.language || 'zh');

    useEffect(() => {
        setAppName(settings.app_name);
        setDateFormat(settings.date_format);
        setTimeFormat(settings.time_format);
        setTimezone(settings.timezone);
        setLanguage(locale || settings.language || 'zh');
    }, [settings, locale]);

    const handleLanguageChange = (nextLang: 'zh' | 'en') => {
        setLanguage(nextLang);
        setLocale(nextLang);
    };

    const formatPreviewDate = (format: string): string => {
        const now = new Date();
        const day = now.getDate().toString().padStart(2, '0');
        const month = (now.getMonth() + 1).toString().padStart(2, '0');
        const year = now.getFullYear().toString();

        switch (format) {
            case 'DD/MM/YYYY': return `${day}/${month}/${year}`;
            case 'YYYY-MM-DD': return `${year}-${month}-${day}`;
            default: return `${month}/${day}/${year}`;
        }
    };

    const formatPreviewTime = (format: string): string => {
        const now = new Date();
        if (format === '24h') {
            return now.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
        }
        return now.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit', hour12: true });
    };

    const handleSave = async () => {
        setIsSaving(true);
        setSaveSuccess(false);

        const success = await updateSettings({
            app_name: appName,
            date_format: dateFormat,
            time_format: timeFormat as '12h' | '24h',
            timezone,
            language,
        });

        // Also save to localStorage immediately
        localStorage.setItem('app_language', language);
        setLocale(language);

        setIsSaving(false);
        if (success) {
            setSaveSuccess(true);
            setTimeout(() => setSaveSuccess(false), 3000);
        }
    };

    return (
        <div className="space-y-4">
            <div className="flex items-center justify-between">
                <div>
                    <h2 className="text-base font-semibold text-gray-900 dark:text-white">{tSettings('title')}</h2>
                    <p className="text-xs text-gray-500 dark:text-gray-400">{tSettings('description')}</p>
                </div>
                <button
                    onClick={handleSave}
                    disabled={!hasChanges || isSaving}
                    className={clsx(
                        "flex items-center px-3.5 py-1.5 rounded-lg text-xs font-medium transition-all",
                        hasChanges && !isSaving
                            ? "bg-primary-600 text-white hover:bg-primary-700"
                            : "bg-gray-100 dark:bg-gray-700 text-gray-400 cursor-not-allowed"
                    )}
                >
                    {isSaving ? (
                        <Loader2 className="w-3.5 h-3.5 mr-1.5 animate-spin" />
                    ) : saveSuccess ? (
                        <Check className="w-3.5 h-3.5 mr-1.5" />
                    ) : (
                        <Save className="w-3.5 h-3.5 mr-1.5" />
                    )}
                    {isSaving ? tCommon('saving') : saveSuccess ? tCommon('saved') : tCommon('save')}
                </button>
            </div>

            {/* Application Name */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-5 py-3 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="text-sm font-medium text-gray-900 dark:text-white">{tSettings('appName')}</h3>
                </div>
                <div className="p-4">
                    <input
                        type="text"
                        value={appName}
                        onChange={(e) => setAppName(e.target.value)}
                        placeholder="Fileyard"
                        className="w-full max-w-md px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
                    />
                    <p className="mt-1.5 text-xs text-gray-500 dark:text-gray-400">
                        {tSettings('appNameDesc')}
                    </p>
                </div>
            </div>

            {/* Language / 语言 */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-5 py-3 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
                    <div className="flex items-center gap-2.5">
                        <div className="p-1.5 bg-primary-50 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400 rounded-lg">
                            <Globe className="w-4 h-4" />
                        </div>
                        <div>
                            <h3 className="text-sm font-medium text-gray-900 dark:text-white">
                                {tSettings('language')}
                            </h3>
                            <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                                {tSettings('languageHint')}
                            </p>
                        </div>
                    </div>
                    {/* storageui 风格的快速选择器 */}
                    <div className="flex items-center gap-2">
                        <select
                            value={language}
                            onChange={(e) => handleLanguageChange(e.target.value as 'zh' | 'en')}
                            className="px-2.5 py-1 text-xs border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 font-medium cursor-pointer shadow-xs"
                        >
                            {LANGUAGES.map((lang) => (
                                <option key={lang.value} value={lang.value}>
                                    {lang.label} ({lang.nativeName})
                                </option>
                            ))}
                        </select>
                    </div>
                </div>

                <div className="p-3 space-y-1.5">
                    {LANGUAGES.map((lang) => (
                        <label
                            key={lang.value}
                            className={clsx(
                                "flex items-center justify-between p-2.5 px-3 rounded-lg border-2 cursor-pointer transition-all",
                                language === lang.value
                                    ? "border-primary-500 bg-primary-50 dark:bg-primary-900/20"
                                    : "border-transparent bg-gray-50 dark:bg-gray-700/50 hover:bg-gray-100 dark:hover:bg-gray-700"
                            )}
                        >
                            <div className="flex items-center gap-3">
                                <input
                                    type="radio"
                                    name="language"
                                    value={lang.value}
                                    checked={language === lang.value}
                                    onChange={(e) => handleLanguageChange(e.target.value as 'zh' | 'en')}
                                    className="w-3.5 h-3.5 text-primary-600 focus:ring-primary-500"
                                />
                                <div>
                                    <p className="text-xs font-medium text-gray-900 dark:text-white">
                                        {lang.label}
                                    </p>
                                    <p className="text-[11px] text-gray-500 dark:text-gray-400">
                                        {lang.nativeName} · {language === 'zh' ? lang.descZh : lang.descEn}
                                    </p>
                                </div>
                            </div>
                            <span className="text-[11px] font-mono font-medium px-2 py-0.5 rounded-md bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-600 text-gray-600 dark:text-gray-300">
                                {lang.code}
                            </span>
                        </label>
                    ))}
                </div>
            </div>

            {/* Date Format */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-5 py-3 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="text-sm font-medium text-gray-900 dark:text-white">{tSettings('dateFormat')}</h3>
                </div>
                <div className="p-3 space-y-1.5">
                    {DATE_FORMATS.map((format) => (
                        <label
                            key={format.value}
                            className={clsx(
                                "flex items-center justify-between p-2.5 px-3 rounded-lg border-2 cursor-pointer transition-all",
                                dateFormat === format.value
                                    ? "border-primary-500 bg-primary-50 dark:bg-primary-900/20"
                                    : "border-transparent bg-gray-50 dark:bg-gray-700/50 hover:bg-gray-100 dark:hover:bg-gray-700"
                            )}
                        >
                            <div className="flex items-center gap-3">
                                <input
                                    type="radio"
                                    name="dateFormat"
                                    value={format.value}
                                    checked={dateFormat === format.value}
                                    onChange={(e) => setDateFormat(e.target.value)}
                                    className="w-3.5 h-3.5 text-primary-600 focus:ring-primary-500"
                                />
                                <div>
                                    <p className="text-xs font-medium text-gray-900 dark:text-white">{format.label}</p>
                                    <p className="text-[11px] text-gray-500 dark:text-gray-400">{format.description}</p>
                                </div>
                            </div>
                            <code className="px-2.5 py-0.5 bg-gray-100 dark:bg-gray-600 rounded text-xs font-mono text-gray-700 dark:text-gray-300">
                                {formatPreviewDate(format.value)}
                            </code>
                        </label>
                    ))}
                </div>
            </div>

            {/* Time Format */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-5 py-3 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="text-sm font-medium text-gray-900 dark:text-white">{tSettings('timeFormat')}</h3>
                </div>
                <div className="p-3">
                    <div className="grid grid-cols-2 gap-3">
                        {TIME_FORMATS.map((format) => (
                            <label
                                key={format.value}
                                className={clsx(
                                    "flex flex-col items-center justify-center p-3 rounded-lg border-2 cursor-pointer transition-all text-center",
                                    timeFormat === format.value
                                        ? "border-primary-500 bg-primary-50 dark:bg-primary-900/20"
                                        : "border-transparent bg-gray-50 dark:bg-gray-700/50 hover:bg-gray-100 dark:hover:bg-gray-700"
                                )}
                            >
                                <input
                                    type="radio"
                                    name="timeFormat"
                                    value={format.value}
                                    checked={timeFormat === format.value}
                                    onChange={(e) => setTimeFormat(e.target.value as '12h' | '24h')}
                                    className="sr-only"
                                />
                                <code className="text-lg font-mono font-bold text-gray-900 dark:text-white mb-1">
                                    {formatPreviewTime(format.value)}
                                </code>
                                <p className="text-xs font-medium text-gray-700 dark:text-gray-300">{format.label}</p>
                            </label>
                        ))}
                    </div>
                </div>
            </div>

            {/* Timezone */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-5 py-3 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="text-sm font-medium text-gray-900 dark:text-white">{tSettings('timezone')}</h3>
                </div>
                <div className="p-4">
                    <select
                        value={timezone}
                        onChange={(e) => setTimezone(e.target.value)}
                        className="w-full max-w-md px-3 py-1.5 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
                    >
                        {TIMEZONES.map((tz) => (
                            <option key={tz.value} value={tz.value}>
                                {tz.label} ({tz.offset})
                            </option>
                        ))}
                    </select>
                    <p className="mt-1.5 text-xs text-gray-500 dark:text-gray-400">
                        {tSettings('timezoneDesc')}
                    </p>
                </div>
            </div>
        </div>
    );
}
