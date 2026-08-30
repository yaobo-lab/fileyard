import { useState, useEffect } from 'react';
import { Save, Check, Loader2, Calendar } from 'lucide-react';
import { useGlobalSettings } from '../../context/GlobalSettingsContext';
import clsx from 'clsx';

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
    
    const [appName, setAppName] = useState(settings.app_name);
    const [dateFormat, setDateFormat] = useState(settings.date_format);
    const [timeFormat, setTimeFormat] = useState(settings.time_format);
    const [timezone, setTimezone] = useState(settings.timezone);
    const [isSaving, setIsSaving] = useState(false);
    const [saveSuccess, setSaveSuccess] = useState(false);
    
    const hasChanges = 
        appName !== settings.app_name ||
        dateFormat !== settings.date_format ||
        timeFormat !== settings.time_format ||
        timezone !== settings.timezone;

    useEffect(() => {
        setAppName(settings.app_name);
        setDateFormat(settings.date_format);
        setTimeFormat(settings.time_format);
        setTimezone(settings.timezone);
    }, [settings]);

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
                    <h2 className="text-lg font-semibold text-gray-900 dark:text-white">General Settings</h2>
                    <p className="text-sm text-gray-500 dark:text-gray-400">Application name and date/time configuration</p>
                </div>
                <button
                    onClick={handleSave}
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
                    {isSaving ? 'Saving...' : saveSuccess ? 'Saved!' : 'Save'}
                </button>
            </div>

            {/* Application Name */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="font-medium text-gray-900 dark:text-white">Application Name</h3>
                </div>
                <div className="p-6">
                    <input
                        type="text"
                        value={appName}
                        onChange={(e) => setAppName(e.target.value)}
                        placeholder="ClovaLink"
                        className="w-full max-w-md px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
                    />
                    <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">
                        Displayed in the browser tab and throughout the application
                    </p>
                </div>
            </div>

            {/* Live Preview Card */}
            <div className="bg-gradient-to-r from-primary-500 to-primary-600 rounded-xl p-6 text-white">
                <div className="flex items-center gap-2 mb-3 opacity-90">
                    <Calendar className="w-4 h-4" />
                    <span className="text-sm font-medium">Date & Time Preview</span>
                </div>
                <div className="text-3xl font-bold tracking-tight">
                    {formatPreviewDate(dateFormat)} {formatPreviewTime(timeFormat)}
                </div>
                <p className="mt-2 text-sm opacity-75">
                    Current date and time in your selected format
                </p>
            </div>

            {/* Date Format */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="font-medium text-gray-900 dark:text-white">Date Format</h3>
                </div>
                <div className="p-4 space-y-2">
                    {DATE_FORMATS.map((format) => (
                        <label
                            key={format.value}
                            className={clsx(
                                "flex items-center justify-between p-4 rounded-lg border-2 cursor-pointer transition-all",
                                dateFormat === format.value
                                    ? "border-primary-500 bg-primary-50 dark:bg-primary-900/20"
                                    : "border-transparent bg-gray-50 dark:bg-gray-700/50 hover:bg-gray-100 dark:hover:bg-gray-700"
                            )}
                        >
                            <div className="flex items-center gap-4">
                                <input
                                    type="radio"
                                    name="dateFormat"
                                    value={format.value}
                                    checked={dateFormat === format.value}
                                    onChange={(e) => setDateFormat(e.target.value)}
                                    className="w-4 h-4 text-primary-600 focus:ring-primary-500"
                                />
                                <div>
                                    <p className="text-sm font-medium text-gray-900 dark:text-white">{format.label}</p>
                                    <p className="text-xs text-gray-500 dark:text-gray-400">{format.description}</p>
                                </div>
                            </div>
                            <code className="px-3 py-1 bg-gray-100 dark:bg-gray-600 rounded text-sm font-mono text-gray-700 dark:text-gray-300">
                                {formatPreviewDate(format.value)}
                            </code>
                        </label>
                    ))}
                </div>
            </div>

            {/* Time Format */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="font-medium text-gray-900 dark:text-white">Time Format</h3>
                </div>
                <div className="p-4">
                    <div className="grid grid-cols-2 gap-4">
                        {TIME_FORMATS.map((format) => (
                            <label
                                key={format.value}
                                className={clsx(
                                    "flex flex-col items-center justify-center p-6 rounded-lg border-2 cursor-pointer transition-all text-center",
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
                                <code className="text-2xl font-mono font-bold text-gray-900 dark:text-white mb-2">
                                    {formatPreviewTime(format.value)}
                                </code>
                                <p className="text-sm font-medium text-gray-700 dark:text-gray-300">{format.label}</p>
                            </label>
                        ))}
                    </div>
                </div>
            </div>

            {/* Timezone */}
            <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm">
                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700">
                    <h3 className="font-medium text-gray-900 dark:text-white">Default Timezone</h3>
                </div>
                <div className="p-6">
                    <select
                        value={timezone}
                        onChange={(e) => setTimezone(e.target.value)}
                        className="w-full max-w-md px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
                    >
                        {TIMEZONES.map((tz) => (
                            <option key={tz.value} value={tz.value}>
                                {tz.label} ({tz.offset})
                            </option>
                        ))}
                    </select>
                    <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">
                        Used as the default for scheduling and timestamps
                    </p>
                </div>
            </div>
        </div>
    );
}
