import React, { useState, useEffect, useMemo } from 'react';
import {
  Radio,
  Search,
  RefreshCw,
  Copy,
  Check,
  PowerOff,
  Info,
  Server,
  Activity,
  Wifi,
  WifiOff,
  Filter,
  X,
  Clock,
  ShieldCheck,
  Layers,
  ChevronRight,
  AlertTriangle,
} from 'lucide-react';
import { useAuthFetch } from '../context/AuthContext';
import { useTranslations } from '../context/I18nContext';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';

export interface MqttClient {
  node_id?: number;
  clientid: string;
  username?: string;
  superuser?: boolean;
  proto_ver?: number;
  ip_address?: string;
  port?: number;
  connected: boolean;
  connected_at?: string;
  disconnected_at?: string;
  disconnected_reason?: string;
  keepalive?: number;
  clean_start?: boolean;
  session_present?: boolean;
  expiry_interval?: number;
  created_at?: string;
  subscriptions_cnt?: number;
  max_subscriptions?: number;
  last_will?: any;
  inflight?: number;
  max_inflight?: number;
  mqueue_len?: number;
  max_mqueue?: number;
}

export function MqttClientsPage() {
  const t = useTranslations();
  const authFetch = useAuthFetch();

  const [clients, setClients] = useState<MqttClient[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [refreshing, setRefreshing] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  // Search & Filter state
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'online' | 'offline'>('all');

  // Active detail modal client
  const [detailClient, setDetailClient] = useState<MqttClient | null>(null);

  // Kick client modal state
  const [kickTarget, setKickTarget] = useState<MqttClient | null>(null);
  const [kicking, setKicking] = useState<boolean>(false);

  // Copied indicator
  const [copiedId, setCopiedId] = useState<string | null>(null);

  const fetchClients = async (isManualRefresh = false) => {
    if (isManualRefresh) {
      setRefreshing(true);
    } else {
      setLoading(true);
    }
    setError(null);

    try {
      // Build query params
      const params = new URLSearchParams();
      params.set('_limit', '1000');
      if (statusFilter === 'online') {
        params.set('connected', 'true');
      } else if (statusFilter === 'offline') {
        params.set('connected', 'false');
      }

      const res = await authFetch(`/api/mqtt/clients?${params.toString()}`);
      if (!res.ok) {
        if (res.status === 503) {
          throw new Error('MQTT 服务未就绪或未启动');
        }
        throw new Error(`获取 MQTT 客户端列表失败 (${res.status})`);
      }

      const data = await res.json();
      if (Array.isArray(data)) {
        setClients(data);
      } else {
        setClients([]);
      }
    } catch (err: any) {
      console.error('Failed to load MQTT clients:', err);
      setError(err.message || '加载 MQTT 客户端时出错');
      setClients([]);
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    fetchClients();
  }, [statusFilter]);

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedId(text);
    setTimeout(() => setCopiedId(null), 2000);
  };

  const handleKickClient = async () => {
    if (!kickTarget) return;
    setKicking(true);
    try {
      const res = await authFetch(`/api/mqtt/clients/${encodeURIComponent(kickTarget.clientid)}`, {
        method: 'DELETE',
      });
      if (!res.ok) {
        throw new Error(`断开客户端失败 (${res.status})`);
      }
      setKickTarget(null);
      // Refresh list
      await fetchClients(true);
    } catch (err: any) {
      alert(err.message || '断开客户端失败');
    } finally {
      setKicking(false);
    }
  };

  // Filter clients based on search query
  const filteredClients = useMemo(() => {
    let result = clients;
    if (searchQuery.trim()) {
      const query = searchQuery.trim().toLowerCase();
      result = result.filter((client) => {
        const idMatch = client.clientid?.toLowerCase().includes(query);
        const userMatch = client.username?.toLowerCase().includes(query);
        const ipMatch = client.ip_address?.toLowerCase().includes(query);
        return idMatch || userMatch || ipMatch;
      });
    }
    return result;
  }, [clients, searchQuery]);

  // Statistics
  const stats = useMemo(() => {
    const total = clients.length;
    const online = clients.filter((c) => c.connected).length;
    const offline = total - online;
    return { total, online, offline };
  }, [clients]);

  const formatProtocolVersion = (ver?: number) => {
    switch (ver) {
      case 3:
        return 'MQTT 3.1';
      case 4:
        return 'MQTT 3.1.1';
      case 5:
        return 'MQTT 5.0';
      default:
        return ver ? `v${ver}` : '-';
    }
  };

  return (
    <div className="flex-1 flex flex-col h-full bg-slate-50/50 dark:bg-slate-950/50 overflow-hidden">
      {/* Top Header */}
      <div className="px-6 py-5 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 flex-shrink-0">
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4">
          <div>
            <div className="flex items-center gap-2.5">
              <div className="p-2 rounded-xl bg-blue-50 dark:bg-blue-950/60 text-blue-600 dark:text-blue-400 border border-blue-100 dark:border-blue-900/50">
                <Radio className="w-5 h-5" />
              </div>
              <h1 className="text-xl font-semibold tracking-tight text-slate-900 dark:text-slate-100">
                {t('MqttClients.title') || '固件编译机'}
              </h1>
            </div>
            <p className="mt-1 text-sm text-slate-500 dark:text-slate-400">
              {t('MqttClients.description') || '查看与管理当前活跃与历史固件编译设备（支持下发编译指令到设备进行编译固件，自动打包，并上传到物联网平台）'}
            </p>
          </div>

          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              onClick={() => fetchClients(true)}
              disabled={loading || refreshing}
              className="h-9 gap-1.5 shadow-sm"
            >
              <RefreshCw className={`w-3.5 h-3.5 ${refreshing ? 'animate-spin text-blue-600' : ''}`} />
              <span>{t('MqttClients.refresh') || '刷新'}</span>
            </Button>
          </div>
        </div>

        {/* Stats row */}
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mt-5">
          <div className="flex items-center gap-3.5 p-3.5 rounded-xl border border-slate-200/80 dark:border-slate-800 bg-slate-50/60 dark:bg-slate-900/40">
            <div className="p-2.5 rounded-lg bg-indigo-50 dark:bg-indigo-950/60 text-indigo-600 dark:text-indigo-400">
              <Server className="w-4 h-4" />
            </div>
            <div>
              <div className="text-xs font-medium text-slate-500 dark:text-slate-400">
                {t('MqttClients.totalClients') || '总客户端数'}
              </div>
              <div className="text-xl font-bold tracking-tight text-slate-900 dark:text-slate-100">
                {stats.total}
              </div>
            </div>
          </div>

          <div className="flex items-center gap-3.5 p-3.5 rounded-xl border border-emerald-100 dark:border-emerald-950/50 bg-emerald-50/40 dark:bg-emerald-950/20">
            <div className="p-2.5 rounded-lg bg-emerald-100/80 dark:bg-emerald-900/50 text-emerald-600 dark:text-emerald-400 relative">
              <Wifi className="w-4 h-4" />
              <span className="absolute top-1.5 right-1.5 w-2 h-2 rounded-full bg-emerald-500 animate-ping opacity-75" />
            </div>
            <div>
              <div className="text-xs font-medium text-emerald-700 dark:text-emerald-400">
                {t('MqttClients.onlineClients') || '在线客户端'}
              </div>
              <div className="text-xl font-bold tracking-tight text-emerald-700 dark:text-emerald-300">
                {stats.online}
              </div>
            </div>
          </div>

          <div className="flex items-center gap-3.5 p-3.5 rounded-xl border border-slate-200/80 dark:border-slate-800 bg-slate-50/60 dark:bg-slate-900/40">
            <div className="p-2.5 rounded-lg bg-slate-100 dark:bg-slate-800 text-slate-500 dark:text-slate-400">
              <WifiOff className="w-4 h-4" />
            </div>
            <div>
              <div className="text-xs font-medium text-slate-500 dark:text-slate-400">
                {t('MqttClients.offlineClients') || '离线客户端'}
              </div>
              <div className="text-xl font-bold tracking-tight text-slate-700 dark:text-slate-300">
                {stats.offline}
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Filter and Search Bar */}
      <div className="px-6 py-3 border-b border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/70 flex-shrink-0 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3 flex-1 min-w-[280px] max-w-md">
          <div className="relative flex-1">
            <Search className="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
            <Input
              type="text"
              placeholder={t('MqttClients.searchPlaceholder') || '输入 Client ID、用户名或 IP 搜索...'}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9 pr-8 h-9 text-sm rounded-lg"
            />
            {searchQuery && (
              <button
                onClick={() => setSearchQuery('')}
                className="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        </div>

        <div className="flex items-center gap-2">
          {/* Status filter tabs */}
          <div className="flex items-center p-0.5 rounded-lg bg-slate-100 dark:bg-slate-800 border border-slate-200/80 dark:border-slate-700 text-xs">
            <button
              onClick={() => setStatusFilter('all')}
              className={`px-3 py-1.5 rounded-md font-medium transition-all ${statusFilter === 'all'
                ? 'bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100 shadow-sm'
                : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200'
                }`}
            >
              {t('MqttClients.allStatus') || '全部'}
            </button>
            <button
              onClick={() => setStatusFilter('online')}
              className={`px-3 py-1.5 rounded-md font-medium transition-all flex items-center gap-1.5 ${statusFilter === 'online'
                ? 'bg-white dark:bg-slate-700 text-emerald-600 dark:text-emerald-400 shadow-sm'
                : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200'
                }`}
            >
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
              {t('MqttClients.online') || '在线'}
            </button>
            <button
              onClick={() => setStatusFilter('offline')}
              className={`px-3 py-1.5 rounded-md font-medium transition-all flex items-center gap-1.5 ${statusFilter === 'offline'
                ? 'bg-white dark:bg-slate-700 text-slate-700 dark:text-slate-300 shadow-sm'
                : 'text-slate-500 dark:text-slate-400 hover:text-slate-800 dark:hover:text-slate-200'
                }`}
            >
              <span className="w-1.5 h-1.5 rounded-full bg-slate-400" />
              {t('MqttClients.offline') || '离线'}
            </button>
          </div>

          {(searchQuery || statusFilter !== 'all') && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setSearchQuery('');
                setStatusFilter('all');
              }}
              className="h-8 text-xs text-slate-500 hover:text-slate-900 dark:hover:text-slate-100"
            >
              {t('MqttClients.reset') || '重置'}
            </Button>
          )}
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-auto p-6">
        {error ? (
          <div className="max-w-xl mx-auto my-12 p-6 rounded-2xl border border-red-200 dark:border-red-900/40 bg-red-50/50 dark:bg-red-950/20 text-center">
            <div className="w-12 h-12 rounded-full bg-red-100 dark:bg-red-900/40 text-red-600 dark:text-red-400 flex items-center justify-center mx-auto mb-3">
              <AlertTriangle className="w-6 h-6" />
            </div>
            <h3 className="text-base font-semibold text-red-900 dark:text-red-200">
              {error}
            </h3>
            <p className="mt-1.5 text-xs text-red-600/80 dark:text-red-400/80">
              请检查 MQTT 服务状态或网络连接
            </p>
            <Button
              variant="outline"
              size="sm"
              onClick={() => fetchClients(true)}
              className="mt-4 border-red-200 dark:border-red-800 text-red-700 dark:text-red-300 hover:bg-red-100/50"
            >
              重试
            </Button>
          </div>
        ) : (
          <div className="rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 overflow-hidden shadow-sm">
            <Table>
              <TableHeader className="bg-slate-50/80 dark:bg-slate-800/50">
                <TableRow>
                  <TableHead className="w-[260px] font-semibold text-xs">
                    {t('MqttClients.clientId') || '设备名称'}
                  </TableHead>
                  <TableHead className="font-semibold text-xs">
                    {t('MqttClients.username') || '用户名'}
                  </TableHead>
                  <TableHead className="w-[110px] font-semibold text-xs">
                    {t('MqttClients.status') || '状态'}
                  </TableHead>
                  <TableHead className="font-semibold text-xs">
                    {t('MqttClients.ipAddress') || 'IP 地址'}
                  </TableHead>
                  <TableHead className="font-semibold text-xs">
                    {t('MqttClients.protoVer') || '协议'}
                  </TableHead>
                  <TableHead className="font-semibold text-xs">
                    {t('MqttClients.keepalive') || '心跳'}
                  </TableHead>
                  <TableHead className="font-semibold text-xs">
                    {t('MqttClients.subscriptions') || '订阅数'}
                  </TableHead>
                  <TableHead className="font-semibold text-xs">
                    {t('MqttClients.connectedAt') || '连接时间'}
                  </TableHead>
                  <TableHead className="text-right w-[140px] font-semibold text-xs">
                    {t('MqttClients.actions') || '操作'}
                  </TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {loading ? (
                  <TableRow>
                    <TableCell colSpan={9} className="h-48 text-center text-slate-400">
                      <div className="flex flex-col items-center justify-center gap-2.5">
                        <RefreshCw className="w-6 h-6 animate-spin text-blue-500" />
                        <span className="text-xs font-medium">正在加载客户端列表...</span>
                      </div>
                    </TableCell>
                  </TableRow>
                ) : filteredClients.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={9} className="h-64 text-center">
                      <div className="flex flex-col items-center justify-center py-6 text-slate-400">
                        <div className="w-10 h-10 rounded-xl bg-slate-100 dark:bg-slate-800 flex items-center justify-center text-slate-400 mb-2.5">
                          <Radio className="w-5 h-5" />
                        </div>
                        <p className="text-sm font-medium text-slate-700 dark:text-slate-300">
                          {t('MqttClients.emptyTitle') || '未找到 固件编译机'}
                        </p>
                        <p className="mt-1 text-xs text-slate-500 dark:text-slate-400 max-w-sm">
                          {searchQuery
                            ? `没有与 “${searchQuery}” 匹配的客户端`
                            : statusFilter !== 'all'
                              ? `当前暂无${statusFilter === 'online' ? '在线' : '离线'}客户端`
                              : t('MqttClients.emptyDesc') || '暂无客户端连接记录'}
                        </p>
                        {(searchQuery || statusFilter !== 'all') && (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => {
                              setSearchQuery('');
                              setStatusFilter('all');
                            }}
                            className="mt-3.5 h-8 text-xs"
                          >
                            清除筛选条件
                          </Button>
                        )}
                      </div>
                    </TableCell>
                  </TableRow>
                ) : (
                  filteredClients.map((client) => (
                    <TableRow
                      key={client.clientid}
                      className="hover:bg-slate-50/70 dark:hover:bg-slate-800/40 transition-colors"
                    >
                      {/* Client ID */}
                      <TableCell className="font-mono text-xs">
                        <div className="flex items-center gap-1.5 group">
                          <span
                            className="font-medium text-slate-800 dark:text-slate-200 truncate max-w-[200px]"
                            title={client.clientid}
                          >
                            {client.clientid}
                          </span>
                          <button
                            onClick={() => handleCopy(client.clientid)}
                            className="text-slate-400 hover:text-slate-600 dark:hover:text-slate-300 opacity-0 group-hover:opacity-100 transition-opacity"
                            title="复制 Client ID"
                          >
                            {copiedId === client.clientid ? (
                              <Check className="w-3.5 h-3.5 text-emerald-500" />
                            ) : (
                              <Copy className="w-3.5 h-3.5" />
                            )}
                          </button>
                        </div>
                      </TableCell>

                      {/* Username */}
                      <TableCell className="text-xs text-slate-600 dark:text-slate-300">
                        {client.username ? (
                          <span className="font-medium">{client.username}</span>
                        ) : (
                          <span className="text-slate-400 italic">-</span>
                        )}
                      </TableCell>

                      {/* Status */}
                      <TableCell>
                        {client.connected ? (
                          <Badge
                            variant="outline"
                            className="bg-emerald-50 text-emerald-700 dark:bg-emerald-950/50 dark:text-emerald-400 border-emerald-200 dark:border-emerald-800/60 gap-1.5 py-0.5 text-[11px] font-medium"
                          >
                            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
                            {t('MqttClients.online') || '在线'}
                          </Badge>
                        ) : (
                          <Badge
                            variant="outline"
                            className="bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400 border-slate-200 dark:border-slate-700 gap-1.5 py-0.5 text-[11px] font-medium"
                          >
                            <span className="w-1.5 h-1.5 rounded-full bg-slate-400" />
                            {t('MqttClients.offline') || '离线'}
                          </Badge>
                        )}
                      </TableCell>

                      {/* IP & Port */}
                      <TableCell className="text-xs font-mono text-slate-600 dark:text-slate-400">
                        {client.ip_address ? (
                          <span>
                            {client.ip_address}
                            {client.port ? `:${client.port}` : ''}
                          </span>
                        ) : (
                          <span className="text-slate-400">-</span>
                        )}
                      </TableCell>

                      {/* Protocol */}
                      <TableCell className="text-xs text-slate-600 dark:text-slate-400">
                        <span className="px-1.5 py-0.5 rounded bg-slate-100 dark:bg-slate-800 text-[11px] font-mono">
                          {formatProtocolVersion(client.proto_ver)}
                        </span>
                      </TableCell>

                      {/* Keepalive */}
                      <TableCell className="text-xs text-slate-600 dark:text-slate-400 font-mono">
                        {client.keepalive !== undefined ? `${client.keepalive}s` : '-'}
                      </TableCell>

                      {/* Subscriptions */}
                      <TableCell className="text-xs text-slate-600 dark:text-slate-400">
                        <span className="font-semibold text-slate-700 dark:text-slate-300">
                          {client.subscriptions_cnt ?? 0}
                        </span>
                      </TableCell>

                      {/* Connected At / Disconnected At */}
                      <TableCell className="text-xs text-slate-500 dark:text-slate-400">
                        {client.connected ? (
                          <span title={`创建时间: ${client.created_at || '-'}`}>
                            {client.connected_at || '-'}
                          </span>
                        ) : (
                          <span className="text-slate-400" title={`断开原因: ${client.disconnected_reason || '-'}`}>
                            {client.disconnected_at || '-'}
                          </span>
                        )}
                      </TableCell>

                      {/* Actions */}
                      <TableCell className="text-right">
                        <div className="flex items-center justify-end gap-1">
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => setDetailClient(client)}
                            className="h-8 px-2 text-xs text-blue-600 hover:text-blue-700 hover:bg-blue-50 dark:hover:bg-blue-950/50"
                            title="查看详情"
                          >
                            <Info className="w-3.5 h-3.5 mr-1" />
                            {t('MqttClients.viewDetails') || '详情'}
                          </Button>

                          {client.connected && (
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => setKickTarget(client)}
                              className="h-8 px-2 text-xs text-red-600 hover:text-red-700 hover:bg-red-50 dark:hover:bg-red-950/50"
                              title="断开连接"
                            >
                              <PowerOff className="w-3.5 h-3.5 mr-1" />
                              {t('MqttClients.kick') || '断开'}
                            </Button>
                          )}
                        </div>
                      </TableCell>
                    </TableRow>
                  ))
                )}
              </TableBody>
            </Table>
          </div>
        )}
      </div>

      {/* Details Modal */}
      {detailClient && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-sm animate-in fade-in duration-200">
          <div className="bg-white dark:bg-slate-900 rounded-2xl border border-slate-200 dark:border-slate-800 shadow-2xl w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden">
            {/* Modal Header */}
            <div className="px-6 py-4 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between">
              <div className="flex items-center gap-2.5">
                <div className="p-2 rounded-xl bg-blue-50 dark:bg-blue-950/60 text-blue-600 dark:text-blue-400">
                  <Radio className="w-4 h-4" />
                </div>
                <div>
                  <h2 className="text-base font-semibold text-slate-900 dark:text-slate-100">
                    {t('MqttClients.clientDetails') || '客户端连接详情'}
                  </h2>
                  <p className="text-xs text-slate-500 font-mono truncate max-w-sm">
                    {detailClient.clientid}
                  </p>
                </div>
              </div>
              <button
                onClick={() => setDetailClient(null)}
                className="p-1.5 rounded-lg text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            {/* Modal Body */}
            <div className="p-6 overflow-y-auto space-y-5 text-xs">
              {/* Basic info grid */}
              <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
                <div className="p-3 rounded-xl bg-slate-50 dark:bg-slate-800/50 border border-slate-100 dark:border-slate-800">
                  <div className="text-slate-400 mb-1">连接状态</div>
                  <div className="font-semibold">
                    {detailClient.connected ? (
                      <span className="text-emerald-600 dark:text-emerald-400 flex items-center gap-1">
                        <span className="w-2 h-2 rounded-full bg-emerald-500" />
                        在线
                      </span>
                    ) : (
                      <span className="text-slate-500 flex items-center gap-1">
                        <span className="w-2 h-2 rounded-full bg-slate-400" />
                        离线
                      </span>
                    )}
                  </div>
                </div>

                <div className="p-3 rounded-xl bg-slate-50 dark:bg-slate-800/50 border border-slate-100 dark:border-slate-800">
                  <div className="text-slate-400 mb-1">用户名</div>
                  <div className="font-medium text-slate-800 dark:text-slate-200 truncate">
                    {detailClient.username || '-'}
                  </div>
                </div>

                <div className="p-3 rounded-xl bg-slate-50 dark:bg-slate-800/50 border border-slate-100 dark:border-slate-800">
                  <div className="text-slate-400 mb-1">协议版本</div>
                  <div className="font-medium font-mono text-slate-800 dark:text-slate-200">
                    {formatProtocolVersion(detailClient.proto_ver)}
                  </div>
                </div>

                <div className="p-3 rounded-xl bg-slate-50 dark:bg-slate-800/50 border border-slate-100 dark:border-slate-800">
                  <div className="text-slate-400 mb-1">IP 与端口</div>
                  <div className="font-medium font-mono text-slate-800 dark:text-slate-200">
                    {detailClient.ip_address
                      ? `${detailClient.ip_address}:${detailClient.port || ''}`
                      : '-'}
                  </div>
                </div>

                <div className="p-3 rounded-xl bg-slate-50 dark:bg-slate-800/50 border border-slate-100 dark:border-slate-800">
                  <div className="text-slate-400 mb-1">心跳</div>
                  <div className="font-medium font-mono text-slate-800 dark:text-slate-200">
                    {detailClient.keepalive !== undefined ? `${detailClient.keepalive}s` : '-'}
                  </div>
                </div>

                <div className="p-3 rounded-xl bg-slate-50 dark:bg-slate-800/50 border border-slate-100 dark:border-slate-800">
                  <div className="text-slate-400 mb-1">订阅主题数</div>
                  <div className="font-medium text-slate-800 dark:text-slate-200">
                    {detailClient.subscriptions_cnt ?? 0}
                  </div>
                </div>
              </div>

              {/* Session details */}
              <div>
                <h3 className="text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider mb-2">
                  会话配置与队列
                </h3>
                <div className="p-3.5 rounded-xl bg-slate-50 dark:bg-slate-800/50 border border-slate-100 dark:border-slate-800 space-y-2">
                  <div className="flex justify-between py-1 border-b border-slate-200/50 dark:border-slate-700/50">
                    <span className="text-slate-500">Clean Start</span>
                    <span className="font-mono text-slate-800 dark:text-slate-200">
                      {String(detailClient.clean_start ?? false)}
                    </span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-slate-200/50 dark:border-slate-700/50">
                    <span className="text-slate-500">Session Present</span>
                    <span className="font-mono text-slate-800 dark:text-slate-200">
                      {String(detailClient.session_present ?? false)}
                    </span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-slate-200/50 dark:border-slate-700/50">
                    <span className="text-slate-500">会话过期时间 (Expiry Interval)</span>
                    <span className="font-mono text-slate-800 dark:text-slate-200">
                      {detailClient.expiry_interval !== undefined ? `${detailClient.expiry_interval}s` : '-'}
                    </span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-slate-200/50 dark:border-slate-700/50">
                    <span className="text-slate-500">消息队列长度 (mqueue_len)</span>
                    <span className="font-mono text-slate-800 dark:text-slate-200">
                      {detailClient.mqueue_len ?? 0} / {detailClient.max_mqueue ?? '-'}
                    </span>
                  </div>
                  <div className="flex justify-between py-1">
                    <span className="text-slate-500">在途消息 (Inflight)</span>
                    <span className="font-mono text-slate-800 dark:text-slate-200">
                      {detailClient.inflight ?? 0} / {detailClient.max_inflight ?? '-'}
                    </span>
                  </div>
                </div>
              </div>

              {/* Time stamps */}
              <div>
                <h3 className="text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider mb-2">
                  时间戳记录
                </h3>
                <div className="p-3.5 rounded-xl bg-slate-50 dark:bg-slate-800/50 border border-slate-100 dark:border-slate-800 space-y-2 font-mono">
                  <div className="flex justify-between py-1 border-b border-slate-200/50 dark:border-slate-700/50">
                    <span className="text-slate-500 font-sans">会话创建时间</span>
                    <span className="text-slate-800 dark:text-slate-200">
                      {detailClient.created_at || '-'}
                    </span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-slate-200/50 dark:border-slate-700/50">
                    <span className="text-slate-500 font-sans">最后连接时间</span>
                    <span className="text-slate-800 dark:text-slate-200">
                      {detailClient.connected_at || '-'}
                    </span>
                  </div>
                  <div className="flex justify-between py-1">
                    <span className="text-slate-500 font-sans">断开时间及原因</span>
                    <span className="text-slate-800 dark:text-slate-200">
                      {detailClient.disconnected_at || '-'}
                      {detailClient.disconnected_reason ? ` (${detailClient.disconnected_reason})` : ''}
                    </span>
                  </div>
                </div>
              </div>

              {/* Last will if exists */}
              {detailClient.last_will && Object.keys(detailClient.last_will).length > 0 && (
                <div>
                  <h3 className="text-xs font-semibold text-slate-700 dark:text-slate-300 uppercase tracking-wider mb-2">
                    遗嘱消息 (Last Will)
                  </h3>
                  <pre className="p-3 rounded-xl bg-slate-900 text-slate-200 font-mono text-[11px] overflow-x-auto">
                    {JSON.stringify(detailClient.last_will, null, 2)}
                  </pre>
                </div>
              )}
            </div>

            {/* Modal Footer */}
            <div className="px-6 py-3.5 border-t border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/50 flex justify-end">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setDetailClient(null)}
                className="h-8 text-xs"
              >
                关闭
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* Kick Confirmation Modal */}
      {kickTarget && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-sm animate-in fade-in duration-200">
          <div className="bg-white dark:bg-slate-900 rounded-2xl border border-slate-200 dark:border-slate-800 shadow-2xl w-full max-w-md overflow-hidden">
            <div className="p-6">
              <div className="w-10 h-10 rounded-full bg-red-100 dark:bg-red-950/60 text-red-600 dark:text-red-400 flex items-center justify-center mb-4">
                <AlertTriangle className="w-5 h-5" />
              </div>
              <h3 className="text-base font-semibold text-slate-900 dark:text-slate-100">
                {t('MqttClients.kickConfirmTitle') || '断开客户端连接'}
              </h3>
              <p className="mt-2 text-xs text-slate-500 dark:text-slate-400">
                确定要断开客户端 <span className="font-mono font-semibold text-slate-800 dark:text-slate-200">{kickTarget.clientid}</span> 的连接吗？该客户端将被踢出当前 Broker 会话。
              </p>
            </div>
            <div className="px-6 py-3.5 border-t border-slate-200 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-900/50 flex justify-end gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setKickTarget(null)}
                disabled={kicking}
                className="h-8 text-xs"
              >
                取消
              </Button>
              <Button
                variant="destructive"
                size="sm"
                onClick={handleKickClient}
                disabled={kicking}
                className="h-8 text-xs gap-1.5"
              >
                {kicking ? <RefreshCw className="w-3.5 h-3.5 animate-spin" /> : <PowerOff className="w-3.5 h-3.5" />}
                <span>确认断开</span>
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default MqttClientsPage;
