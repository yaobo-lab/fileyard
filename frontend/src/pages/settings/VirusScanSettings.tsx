import { useState, useEffect } from 'react';
import {
  Save,
  Check,
  Loader2,
  Shield,
  ShieldCheck,
  ShieldAlert,
  ShieldOff,
  RefreshCw,
  AlertTriangle,
  CheckCircle,
  XCircle,
  FileWarning,
  Search,
  Activity,
  Clock,
  Trash2,
  RotateCcw,
  Eye,
  User,
  ChevronDown,
  Folder,
} from 'lucide-react';
import { useAuthFetch, useAuth } from '../../context/AuthContext';
import clsx from 'clsx';

interface TenantScanSettings {
  tenant_id: string;
  enabled: boolean;
  file_types: string[];
  max_file_size_mb: number;
  action_on_detect: string;
  notify_admin: boolean;
  notify_uploader: boolean;
  auto_suspend_uploader: boolean;
  suspend_threshold: number;
}

interface ScanMetrics {
  enabled: boolean;
  clamd_connected: boolean;
  clamd_version: string | null;
  pending_jobs: number;
  scanning_jobs: number;
  failed_jobs: number;
  scans_last_hour: number;
  infections_last_hour: number;
  avg_scan_duration_ms: number | null;
  total_bytes_scanned_last_hour: number;
}

interface ScanResult {
  id: string;
  file_id: string;
  file_name: string;
  scan_status: string;
  threat_name: string | null;
  scan_duration_ms: number | null;
  scanned_at: string;
}

interface QuarantinedFile {
  id: string;
  file_id: string;
  file_name: string;
  original_path: string;
  threat_name: string;
  original_size: number;
  quarantined_at: string;
  uploader_id: string | null;
  uploader_name: string | null;
  uploader_email: string | null;
}

interface QuarantineResponse {
  items: QuarantinedFile[];
  total: number;
  limit: number;
  offset: number;
}

export function VirusScanSettings() {
  const authFetch = useAuthFetch();
  const { user } = useAuth();
  const isSuperAdmin = user?.role === 'SuperAdmin';

  // Settings state
  const [settings, setSettings] = useState<TenantScanSettings | null>(null);
  const [metrics, setMetrics] = useState<ScanMetrics | null>(null);
  const [scanResults, setScanResults] = useState<ScanResult[]>([]);
  const [quarantinedFiles, setQuarantinedFiles] = useState<QuarantinedFile[]>([]);

  // Pagination state
  const PAGE_SIZE = 10;
  
  // Quarantine pagination
  const [quarantineTotal, setQuarantineTotal] = useState(0);
  const [quarantineOffset, setQuarantineOffset] = useState(0);
  const [loadingMoreQuarantine, setLoadingMoreQuarantine] = useState(false);
  
  // Scan history pagination
  const [historyTotal, setHistoryTotal] = useState(0);
  const [historyOffset, setHistoryOffset] = useState(0);
  const [loadingMoreHistory, setLoadingMoreHistory] = useState(false);

  // Form state
  const [enabled, setEnabled] = useState(false);
  const [fileTypes, setFileTypes] = useState('');
  const [maxFileSizeMb, setMaxFileSizeMb] = useState(100);
  const [actionOnDetect, setActionOnDetect] = useState('flag');
  const [notifyAdmin, setNotifyAdmin] = useState(true);
  const [notifyUploader, setNotifyUploader] = useState(false);
  const [autoSuspendUploader, setAutoSuspendUploader] = useState(false);
  const [suspendThreshold, setSuspendThreshold] = useState(1);

  // UI state
  const [loading, setLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeTab, setActiveTab] = useState<'settings' | 'history' | 'quarantine'>('settings');

  const hasChanges =
    settings &&
    (enabled !== settings.enabled ||
      fileTypes !== settings.file_types.join(', ') ||
      maxFileSizeMb !== settings.max_file_size_mb ||
      actionOnDetect !== settings.action_on_detect ||
      notifyAdmin !== settings.notify_admin ||
      notifyUploader !== settings.notify_uploader ||
      autoSuspendUploader !== settings.auto_suspend_uploader ||
      suspendThreshold !== settings.suspend_threshold);

  // Fetch all data
  const fetchData = async () => {
    setLoading(true);
    setError(null);

    try {
      const [settingsRes, metricsRes] = await Promise.all([
        authFetch('/api/admin/virus-scan/settings'),
        authFetch('/api/admin/virus-scan/metrics'),
      ]);

      if (settingsRes.ok) {
        const s: TenantScanSettings = await settingsRes.json();
        setSettings(s);
        setEnabled(s.enabled);
        setFileTypes(s.file_types.join(', '));
        setMaxFileSizeMb(s.max_file_size_mb);
        setActionOnDetect(s.action_on_detect);
        setNotifyAdmin(s.notify_admin);
        setNotifyUploader(s.notify_uploader);
        setAutoSuspendUploader(s.auto_suspend_uploader);
        setSuspendThreshold(s.suspend_threshold);
      }

      if (metricsRes.ok) {
        setMetrics(await metricsRes.json());
      }
    } catch (err) {
      setError('Failed to load virus scan settings');
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const fetchHistory = async (reset = true) => {
    try {
      const offset = reset ? 0 : historyOffset;
      const res = await authFetch(`/api/admin/virus-scan/results?limit=${PAGE_SIZE}&offset=${offset}`);
      if (res.ok) {
        const data = await res.json();
        // API returns { items, total, limit, offset } for paginated response
        if (data.items) {
          if (reset) {
            setScanResults(data.items);
            setHistoryOffset(PAGE_SIZE);
          } else {
            setScanResults((prev) => [...prev, ...data.items]);
            setHistoryOffset((prev) => prev + PAGE_SIZE);
          }
          setHistoryTotal(data.total);
        } else {
          // Fallback for non-paginated response
          setScanResults(data);
          setHistoryTotal(data.length);
        }
      }
    } catch (err) {
      console.error('Failed to fetch scan history:', err);
    }
  };

  const loadMoreHistory = async () => {
    setLoadingMoreHistory(true);
    await fetchHistory(false);
    setLoadingMoreHistory(false);
  };

  const hasMoreHistory = scanResults.length < historyTotal;

  const fetchQuarantine = async (reset = true) => {
    try {
      const offset = reset ? 0 : quarantineOffset;
      const res = await authFetch(`/api/admin/virus-scan/quarantine?limit=${PAGE_SIZE}&offset=${offset}`);
      if (res.ok) {
        const data: QuarantineResponse = await res.json();
        if (reset) {
          setQuarantinedFiles(data.items);
          setQuarantineOffset(PAGE_SIZE);
        } else {
          setQuarantinedFiles((prev) => [...prev, ...data.items]);
          setQuarantineOffset((prev) => prev + PAGE_SIZE);
        }
        setQuarantineTotal(data.total);
      }
    } catch (err) {
      console.error('Failed to fetch quarantine:', err);
    }
  };

  const loadMoreQuarantine = async () => {
    setLoadingMoreQuarantine(true);
    await fetchQuarantine(false);
    setLoadingMoreQuarantine(false);
  };

  const hasMoreQuarantine = quarantinedFiles.length < quarantineTotal;

  useEffect(() => {
    fetchData();
  }, []);

  useEffect(() => {
    if (activeTab === 'history') {
      fetchHistory();
    } else if (activeTab === 'quarantine') {
      fetchQuarantine();
    }
  }, [activeTab]);

  const handleSave = async () => {
    setIsSaving(true);
    setSaveSuccess(false);
    setError(null);

    try {
      const res = await authFetch('/api/admin/virus-scan/settings', {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          enabled,
          file_types: fileTypes
            .split(',')
            .map((t) => t.trim().toLowerCase())
            .filter(Boolean),
          max_file_size_mb: maxFileSizeMb,
          action_on_detect: actionOnDetect,
          notify_admin: notifyAdmin,
          notify_uploader: notifyUploader,
          auto_suspend_uploader: autoSuspendUploader,
          suspend_threshold: suspendThreshold,
        }),
      });

      if (res.ok) {
        const updated: TenantScanSettings = await res.json();
        setSettings(updated);
        setSaveSuccess(true);
        setTimeout(() => setSaveSuccess(false), 3000);
      } else {
        throw new Error('Failed to save settings');
      }
    } catch (err) {
      setError('Failed to save settings');
    } finally {
      setIsSaving(false);
    }
  };

  const handleDeleteQuarantined = async (id: string) => {
    if (!confirm('Are you sure you want to permanently delete this file? This action cannot be undone.')) {
      return;
    }

    try {
      const res = await authFetch(`/api/admin/virus-scan/quarantine/${id}`, {
        method: 'DELETE',
      });

      if (res.ok) {
        setQuarantinedFiles((prev) => prev.filter((f) => f.id !== id));
      }
    } catch (err) {
      console.error('Failed to delete quarantined file:', err);
    }
  };

  const formatBytes = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-12">
        <Loader2 className="w-8 h-8 animate-spin text-primary-500" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Virus Scanning</h2>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            Configure ClamAV virus scanning for uploaded files
          </p>
        </div>
        {activeTab === 'settings' && (
          <button
            onClick={handleSave}
            disabled={!hasChanges || isSaving}
            className={clsx(
              'flex items-center px-4 py-2 rounded-lg text-sm font-medium transition-all',
              hasChanges && !isSaving
                ? 'bg-primary-600 text-white hover:bg-primary-700'
                : 'bg-gray-100 dark:bg-gray-700 text-gray-400 cursor-not-allowed'
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
        )}
      </div>

      {error && (
        <div className="p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-red-700 dark:text-red-300 text-sm">
          {error}
        </div>
      )}

      {/* Status Card */}
      <div
        className={clsx(
          'rounded-xl border shadow-sm overflow-hidden',
          !metrics?.enabled
            ? 'bg-gray-50 dark:bg-gray-800/50 border-gray-200 dark:border-gray-700'
            : metrics?.clamd_connected
            ? 'bg-green-50 dark:bg-green-900/10 border-green-200 dark:border-green-800'
            : 'bg-yellow-50 dark:bg-yellow-900/10 border-yellow-200 dark:border-yellow-800'
        )}
      >
        <div className="px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div
              className={clsx(
                'p-3 rounded-xl',
                !metrics?.enabled
                  ? 'bg-gray-100 dark:bg-gray-700'
                  : metrics?.clamd_connected
                  ? 'bg-green-100 dark:bg-green-900/30'
                  : 'bg-yellow-100 dark:bg-yellow-900/30'
              )}
            >
              {!metrics?.enabled ? (
                <ShieldOff className="w-6 h-6 text-gray-400" />
              ) : metrics?.clamd_connected ? (
                <ShieldCheck className="w-6 h-6 text-green-600 dark:text-green-400" />
              ) : (
                <ShieldAlert className="w-6 h-6 text-yellow-600 dark:text-yellow-400" />
              )}
            </div>
            <div>
              <h3 className="font-semibold text-gray-900 dark:text-white">
                {!metrics?.enabled
                  ? 'Virus Scanning Disabled'
                  : metrics?.clamd_connected
                  ? 'ClamAV Connected'
                  : 'ClamAV Disconnected'}
              </h3>
              <p className="text-sm text-gray-500 dark:text-gray-400">
                {!metrics?.enabled
                  ? 'Enable scanning in your environment configuration'
                  : metrics?.clamd_version
                  ? `Version: ${metrics.clamd_version}`
                  : 'Waiting for ClamAV daemon...'}
              </p>
            </div>
          </div>
          <button
            onClick={fetchData}
            className="p-2 rounded-lg hover:bg-white/50 dark:hover:bg-gray-700/50 transition-colors"
          >
            <RefreshCw className="w-5 h-5 text-gray-500" />
          </button>
        </div>

        {/* Metrics Grid */}
        {metrics?.enabled && (
          <div className="px-6 py-4 border-t border-gray-200 dark:border-gray-700 grid grid-cols-2 md:grid-cols-4 gap-4">
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">Scans (1h)</p>
              <p className="text-xl font-bold text-gray-900 dark:text-white">{metrics.scans_last_hour}</p>
            </div>
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">Threats (1h)</p>
              <p
                className={clsx(
                  'text-xl font-bold',
                  metrics.infections_last_hour > 0 ? 'text-red-600 dark:text-red-400' : 'text-gray-900 dark:text-white'
                )}
              >
                {metrics.infections_last_hour}
              </p>
            </div>
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">Pending</p>
              <p className="text-xl font-bold text-gray-900 dark:text-white">{metrics.pending_jobs}</p>
            </div>
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">Avg Time</p>
              <p className="text-xl font-bold text-gray-900 dark:text-white">
                {metrics.avg_scan_duration_ms ? `${Math.round(metrics.avg_scan_duration_ms)}ms` : '—'}
              </p>
            </div>
          </div>
        )}
      </div>

      {/* Tabs */}
      <div className="border-b border-gray-200 dark:border-gray-700">
        <nav className="flex gap-4">
          {[
            { id: 'settings', label: 'Settings', icon: Shield },
            { id: 'history', label: 'Scan History', icon: Activity },
            { id: 'quarantine', label: 'Quarantine', icon: FileWarning },
          ].map((tab) => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id as typeof activeTab)}
              className={clsx(
                'flex items-center gap-2 px-3 py-2.5 border-b-2 text-sm font-medium transition-colors',
                activeTab === tab.id
                  ? 'border-primary-500 text-primary-600 dark:text-primary-400'
                  : 'border-transparent text-gray-500 hover:text-gray-700 dark:hover:text-gray-300'
              )}
            >
              <tab.icon className="w-4 h-4" />
              {tab.label}
            </button>
          ))}
        </nav>
      </div>

      {/* Settings Tab */}
      {activeTab === 'settings' && (
        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
          <div className="p-6 space-y-6">
            {/* Enable Toggle */}
            <div className="flex items-center justify-between">
              <div>
                <label className="font-medium text-gray-900 dark:text-white">Enable Virus Scanning</label>
                <p className="text-sm text-gray-500 dark:text-gray-400">
                  Automatically scan uploaded files for malware
                </p>
              </div>
              <button
                onClick={() => setEnabled(!enabled)}
                disabled={!metrics?.enabled}
                className={clsx(
                  'relative inline-flex h-6 w-11 items-center rounded-full transition-colors',
                  !metrics?.enabled
                    ? 'bg-gray-200 dark:bg-gray-700 cursor-not-allowed'
                    : enabled
                    ? 'bg-primary-600'
                    : 'bg-gray-300 dark:bg-gray-600'
                )}
              >
                <span
                  className={clsx(
                    'inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform',
                    enabled ? 'translate-x-6' : 'translate-x-1'
                  )}
                />
              </button>
            </div>

            {!metrics?.enabled && (
              <div className="p-4 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg text-sm text-yellow-700 dark:text-yellow-300">
                <div className="flex items-start gap-3">
                  <AlertTriangle className="w-5 h-5 flex-shrink-0 mt-0.5" />
                  <div>
                    <p className="font-medium">ClamAV is not enabled</p>
                    <p className="mt-1 text-yellow-600 dark:text-yellow-400">
                      To enable virus scanning, set <code className="px-1 py-0.5 bg-yellow-100 dark:bg-yellow-900/50 rounded">CLAMAV_ENABLED=true</code> in your environment configuration.
                    </p>
                  </div>
                </div>
              </div>
            )}

            {/* File Types */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                File Types to Scan
              </label>
              <input
                type="text"
                value={fileTypes}
                onChange={(e) => setFileTypes(e.target.value)}
                placeholder="pdf, doc, docx, xls, xlsx, zip (leave empty to scan all)"
                className="w-full px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500"
              />
              <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">
                Comma-separated list of file extensions. Leave empty to scan all file types.
              </p>
            </div>

            {/* Max File Size */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Maximum File Size (MB)
              </label>
              <input
                type="number"
                value={maxFileSizeMb}
                onChange={(e) => setMaxFileSizeMb(parseInt(e.target.value) || 100)}
                min={1}
                max={500}
                className="w-32 px-4 py-2.5 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-primary-500"
              />
              <p className="mt-2 text-xs text-gray-500 dark:text-gray-400">
                Files larger than this will skip scanning.
              </p>
            </div>

            {/* Action on Detect */}
            <div>
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                Action on Threat Detection
              </label>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
                {[
                  { value: 'flag', label: 'Flag Only', desc: 'Mark file as infected but keep it accessible', icon: AlertTriangle },
                  { value: 'quarantine', label: 'Quarantine', desc: 'Move to quarantine folder, block access', icon: FileWarning },
                  { value: 'delete', label: 'Delete', desc: 'Permanently delete the infected file', icon: Trash2 },
                ].map((opt) => (
                  <button
                    key={opt.value}
                    onClick={() => setActionOnDetect(opt.value)}
                    className={clsx(
                      'p-4 rounded-lg border-2 text-left transition-all',
                      actionOnDetect === opt.value
                        ? 'border-primary-500 bg-primary-50 dark:bg-primary-900/20'
                        : 'border-gray-200 dark:border-gray-600 hover:border-gray-300 dark:hover:border-gray-500'
                    )}
                  >
                    <opt.icon
                      className={clsx(
                        'w-5 h-5 mb-2',
                        actionOnDetect === opt.value ? 'text-primary-600 dark:text-primary-400' : 'text-gray-400'
                      )}
                    />
                    <p
                      className={clsx(
                        'font-medium',
                        actionOnDetect === opt.value ? 'text-primary-700 dark:text-primary-300' : 'text-gray-900 dark:text-white'
                      )}
                    >
                      {opt.label}
                    </p>
                    <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">{opt.desc}</p>
                  </button>
                ))}
              </div>
            </div>

            {/* Notifications */}
            <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
              <h4 className="font-medium text-gray-900 dark:text-white mb-4">Notifications</h4>
              <div className="space-y-4">
                <label className="flex items-center gap-3">
                  <input
                    type="checkbox"
                    checked={notifyAdmin}
                    onChange={(e) => setNotifyAdmin(e.target.checked)}
                    className="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                  />
                  <div>
                    <p className="text-sm font-medium text-gray-900 dark:text-white">Notify Administrators</p>
                    <p className="text-xs text-gray-500 dark:text-gray-400">
                      Send email alerts to admins when threats are detected
                    </p>
                  </div>
                </label>
                <label className="flex items-center gap-3">
                  <input
                    type="checkbox"
                    checked={notifyUploader}
                    onChange={(e) => setNotifyUploader(e.target.checked)}
                    className="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                  />
                  <div>
                    <p className="text-sm font-medium text-gray-900 dark:text-white">Notify File Uploader</p>
                    <p className="text-xs text-gray-500 dark:text-gray-400">
                      Inform the user who uploaded the file about the detection
                    </p>
                  </div>
                </label>
              </div>
            </div>

            {/* Auto-Suspend Section */}
            <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
              <h4 className="font-medium text-gray-900 dark:text-white mb-4">User Suspension</h4>
              <div className="space-y-4">
                <label className="flex items-center gap-3">
                  <input
                    type="checkbox"
                    checked={autoSuspendUploader}
                    onChange={(e) => setAutoSuspendUploader(e.target.checked)}
                    className="w-4 h-4 text-primary-600 border-gray-300 rounded focus:ring-primary-500"
                  />
                  <div>
                    <p className="text-sm font-medium text-gray-900 dark:text-white">Auto-Suspend Uploaders</p>
                    <p className="text-xs text-gray-500 dark:text-gray-400">
                      Automatically suspend users who upload infected files
                    </p>
                  </div>
                </label>

                {autoSuspendUploader && (
                  <div className="ml-7 p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg space-y-4">
                    <div>
                      <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                        Suspension Threshold
                      </label>
                      <div className="flex items-center gap-3">
                        <input
                          type="number"
                          min="1"
                          max="10"
                          value={suspendThreshold}
                          onChange={(e) => setSuspendThreshold(Math.max(1, parseInt(e.target.value) || 1))}
                          className="w-20 px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white text-sm focus:ring-2 focus:ring-primary-500 focus:border-primary-500"
                        />
                        <span className="text-sm text-gray-600 dark:text-gray-400">
                          {suspendThreshold === 1 
                            ? 'Suspend immediately on first offense' 
                            : `Suspend after ${suspendThreshold} infected uploads`}
                        </span>
                      </div>
                    </div>
                    <div className="flex items-start gap-2 p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-700 rounded-lg">
                      <AlertTriangle className="w-4 h-4 text-amber-500 mt-0.5 flex-shrink-0" />
                      <p className="text-xs text-amber-700 dark:text-amber-300">
                        <strong>Note:</strong> Admins and SuperAdmins are exempt from auto-suspension.
                        Suspended users will be unable to log in until manually reinstated.
                      </p>
                    </div>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Scan History Tab */}
      {activeTab === 'history' && (
        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
            <h3 className="font-medium text-gray-900 dark:text-white">Recent Scans</h3>
            <button
              onClick={() => fetchHistory(true)}
              className="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
            >
              <RefreshCw className="w-4 h-4 text-gray-500" />
            </button>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead className="bg-gray-50 dark:bg-gray-700/50">
                <tr>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    File
                  </th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    Status
                  </th>
                  <th className="px-4 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    Threat
                  </th>
                  <th className="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    Duration
                  </th>
                  <th className="px-4 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    Scanned
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                {scanResults.length > 0 ? (
                  scanResults.map((result) => (
                    <tr key={result.id} className="hover:bg-gray-50 dark:hover:bg-gray-700/30">
                      <td className="px-4 py-3 text-sm text-gray-900 dark:text-white truncate max-w-[200px]">
                        {result.file_name}
                      </td>
                      <td className="px-4 py-3">
                        <span
                          className={clsx(
                            'inline-flex items-center gap-1 px-2 py-1 rounded text-xs font-medium',
                            result.scan_status === 'clean'
                              ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
                              : result.scan_status === 'infected'
                              ? 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'
                              : 'bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300'
                          )}
                        >
                          {result.scan_status === 'clean' && <CheckCircle className="w-3 h-3" />}
                          {result.scan_status === 'infected' && <XCircle className="w-3 h-3" />}
                          {result.scan_status}
                        </span>
                      </td>
                      <td className="px-4 py-3 text-sm text-gray-500 dark:text-gray-400">
                        {result.threat_name || '—'}
                      </td>
                      <td className="px-4 py-3 text-right text-sm text-gray-500 dark:text-gray-400">
                        {result.scan_duration_ms ? `${result.scan_duration_ms}ms` : '—'}
                      </td>
                      <td className="px-4 py-3 text-right text-sm text-gray-500 dark:text-gray-400">
                        {new Date(result.scanned_at).toLocaleString(undefined, {
                          month: 'short',
                          day: 'numeric',
                          hour: '2-digit',
                          minute: '2-digit',
                        })}
                      </td>
                    </tr>
                  ))
                ) : (
                  <tr>
                    <td colSpan={5} className="px-4 py-8 text-center text-gray-500 dark:text-gray-400">
                      <Search className="w-8 h-8 mx-auto mb-2 opacity-50" />
                      No scan history available
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
          
          {/* Pagination */}
          {scanResults.length > 0 && (
            <div className="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex items-center justify-between">
              <span className="text-sm text-gray-500 dark:text-gray-400">
                Showing {scanResults.length} of {historyTotal} scans
              </span>
              {hasMoreHistory && (
                <button
                  onClick={loadMoreHistory}
                  disabled={loadingMoreHistory}
                  className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-primary-600 dark:text-primary-400 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-lg transition-colors disabled:opacity-50"
                >
                  {loadingMoreHistory ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      Loading...
                    </>
                  ) : (
                    <>
                      <ChevronDown className="w-4 h-4" />
                      Load More
                    </>
                  )}
                </button>
              )}
            </div>
          )}
        </div>
      )}

      {/* Quarantine Tab */}
      {activeTab === 'quarantine' && (
        <div className="bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between">
            <div>
              <h3 className="font-medium text-gray-900 dark:text-white">Quarantined Files</h3>
              <p className="text-sm text-gray-500 dark:text-gray-400">
                Files detected as malicious and moved to quarantine
              </p>
            </div>
            <button
              onClick={() => fetchQuarantine(true)}
              className="p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
            >
              <RefreshCw className="w-4 h-4 text-gray-500" />
            </button>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full table-fixed">
              <thead className="bg-gray-50 dark:bg-gray-700/50">
                <tr>
                  <th className="w-[25%] px-3 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    File
                  </th>
                  <th className="w-[22%] px-3 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    Uploader
                  </th>
                  <th className="w-[20%] px-3 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    Threat
                  </th>
                  <th className="w-[10%] px-3 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    Size
                  </th>
                  <th className="w-[15%] px-3 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    Date
                  </th>
                  <th className="w-[8%] px-3 py-3 text-right text-xs font-medium text-gray-500 dark:text-gray-400 uppercase">
                    
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-200 dark:divide-gray-700">
                {quarantinedFiles.length > 0 ? (
                  quarantinedFiles.map((file) => (
                    <tr key={file.id} className="hover:bg-gray-50 dark:hover:bg-gray-700/30">
                      <td className="px-3 py-3">
                        <div className="flex flex-col min-w-0">
                          <span className="text-sm font-medium text-gray-900 dark:text-white truncate" title={file.file_name}>
                            {file.file_name}
                          </span>
                          {file.original_path && (
                            <span className="text-xs text-gray-500 dark:text-gray-400 truncate flex items-center gap-1" title={file.original_path}>
                              <Folder className="w-3 h-3 flex-shrink-0" />
                              {file.original_path || '/'}
                            </span>
                          )}
                        </div>
                      </td>
                      <td className="px-3 py-3">
                        {file.uploader_email ? (
                          <div className="flex items-center gap-2 min-w-0">
                            <div className="w-6 h-6 rounded-full bg-gray-200 dark:bg-gray-600 flex items-center justify-center flex-shrink-0">
                              <User className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400" />
                            </div>
                            <div className="flex flex-col min-w-0">
                              <span className="text-sm text-gray-900 dark:text-white truncate" title={file.uploader_name || ''}>
                                {file.uploader_name || 'Unknown'}
                              </span>
                              <span className="text-xs text-gray-500 dark:text-gray-400 truncate" title={file.uploader_email}>
                                {file.uploader_email}
                              </span>
                            </div>
                          </div>
                        ) : (
                          <span className="text-sm text-gray-400 dark:text-gray-500 italic">Unknown</span>
                        )}
                      </td>
                      <td className="px-3 py-3">
                        <span className="inline-flex items-center gap-1 px-2 py-1 rounded text-xs font-medium bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400 truncate max-w-full" title={file.threat_name}>
                          <ShieldAlert className="w-3 h-3 flex-shrink-0" />
                          <span className="truncate">{file.threat_name}</span>
                        </span>
                      </td>
                      <td className="px-3 py-3 text-right text-sm text-gray-500 dark:text-gray-400">
                        {formatBytes(file.original_size)}
                      </td>
                      <td className="px-3 py-3 text-right text-sm text-gray-500 dark:text-gray-400">
                        {new Date(file.quarantined_at).toLocaleString(undefined, {
                          month: 'short',
                          day: 'numeric',
                        })}
                      </td>
                      <td className="px-3 py-3 text-right">
                        <button
                          onClick={() => handleDeleteQuarantined(file.id)}
                          className="p-1.5 rounded-lg text-red-600 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors"
                          title="Delete permanently"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </td>
                    </tr>
                  ))
                ) : (
                  <tr>
                    <td colSpan={6} className="px-4 py-8 text-center text-gray-500 dark:text-gray-400">
                      <ShieldCheck className="w-8 h-8 mx-auto mb-2 text-green-500" />
                      No files in quarantine
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
          
          {/* Pagination */}
          {quarantinedFiles.length > 0 && (
            <div className="px-6 py-4 border-t border-gray-200 dark:border-gray-700 flex items-center justify-between">
              <span className="text-sm text-gray-500 dark:text-gray-400">
                Showing {quarantinedFiles.length} of {quarantineTotal} files
              </span>
              {hasMoreQuarantine && (
                <button
                  onClick={loadMoreQuarantine}
                  disabled={loadingMoreQuarantine}
                  className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-primary-600 dark:text-primary-400 hover:bg-primary-50 dark:hover:bg-primary-900/20 rounded-lg transition-colors disabled:opacity-50"
                >
                  {loadingMoreQuarantine ? (
                    <>
                      <Loader2 className="w-4 h-4 animate-spin" />
                      Loading...
                    </>
                  ) : (
                    <>
                      <ChevronDown className="w-4 h-4" />
                      Load More
                    </>
                  )}
                </button>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

