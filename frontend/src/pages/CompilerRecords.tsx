import React, { useState, useEffect, useMemo } from 'react';
import {
  History,
  Search,
  RefreshCw,
  Copy,
  Check,
  Info,
  AlertTriangle,
  Plus,
  X,
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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

export interface CompilerClientRecord {
  id: string;
  username: string;
  client_id: string;
  device_name?: string;
  ip_address?: string;
  port?: number;
  proto_ver?: number;
  keepalive?: number;
  clean_start?: boolean;
  online: boolean;
  connected_at: string;
  disconnected_at?: string;
  disconnected_reason?: string;
  created_at: string;
  updated_at: string;
}

export function CompilerRecordsPage() {
  const t = useTranslations();
  const authFetch = useAuthFetch();

  const [records, setRecords] = useState<CompilerClientRecord[]>([]);
  const [loading, setLoading] = useState<boolean>(true);
  const [refreshing, setRefreshing] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  // Search & Filter state
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'online' | 'offline'>('all');

  // Detail modal target
  const [detailRecord, setDetailRecord] = useState<CompilerClientRecord | null>(null);

  // Copied indicator
  const [copiedText, setCopiedText] = useState<string | null>(null);

  // Add Compiler Modal state
  const [showAddModal, setShowAddModal] = useState<boolean>(false);
  const [adding, setAdding] = useState<boolean>(false);
  const [addError, setAddError] = useState<string | null>(null);
  const [addForm, setAddForm] = useState({
    username: '',
    clientId: '',
    deviceName: '',
    ipAddress: '',
    port: '1883',
    protoVer: '5',
    keepalive: '60',
    online: false,
  });

  const handleAddCompiler = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!addForm.username.trim()) {
      setAddError('请输入 MQTT 账号 (用户名)');
      return;
    }
    if (!addForm.clientId.trim()) {
      setAddError('请输入 Client ID');
      return;
    }

    setAdding(true);
    setAddError(null);
    try {
      const res = await authFetch('/api/mqtt/database-clients', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          username: addForm.username.trim(),
          client_id: addForm.clientId.trim(),
          device_name: addForm.deviceName.trim() || undefined,
          ip_address: addForm.ipAddress.trim() || undefined,
          port: addForm.port ? parseInt(addForm.port, 10) : undefined,
          proto_ver: addForm.protoVer ? parseInt(addForm.protoVer, 10) : 5,
          keepalive: addForm.keepalive ? parseInt(addForm.keepalive, 10) : 60,
          online: addForm.online,
        }),
      });

      if (!res.ok) {
        let msg = '添加编译机失败';
        try {
          const errData = await res.text();
          if (errData) {
            try {
              const parsed = JSON.parse(errData);
              msg = parsed.message || parsed.error || errData;
            } catch {
              msg = errData;
            }
          }
        } catch {}
        throw new Error(msg);
      }

      setShowAddModal(false);
      setAddForm({
        username: '',
        clientId: '',
        deviceName: '',
        ipAddress: '',
        port: '1883',
        protoVer: '5',
        keepalive: '60',
        online: false,
      });
      await fetchRecords(true);
    } catch (err: any) {
      setAddError(err.message || '添加编译机失败');
    } finally {
      setAdding(false);
    }
  };

  const fetchRecords = async (isManual = false) => {
    if (isManual) {
      setRefreshing(true);
    } else {
      setLoading(true);
    }
    setError(null);

    try {
      const params = new URLSearchParams();
      if (statusFilter === 'online') {
        params.set('online', 'true');
      } else if (statusFilter === 'offline') {
        params.set('online', 'false');
      }

      const res = await authFetch(`/api/mqtt/database-clients?${params.toString()}`);
      if (!res.ok) {
        throw new Error(`获取历史编译机记录失败 (${res.status})`);
      }

      const data = await res.json();
      if (Array.isArray(data)) {
        setRecords(data);
      } else {
        setRecords([]);
      }
    } catch (err: any) {
      console.error('Failed to load compiler records:', err);
      setError(err.message || '加载历史编译机记录失败');
      setRecords([]);
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    fetchRecords();
  }, [statusFilter]);

  const handleCopy = (text: string) => {
    navigator.clipboard.writeText(text);
    setCopiedText(text);
    setTimeout(() => setCopiedText(null), 2000);
  };

  // Filter records based on search input
  const filteredRecords = useMemo(() => {
    let result = records;
    if (searchQuery.trim()) {
      const query = searchQuery.trim().toLowerCase();
      result = result.filter((item) => {
        const idMatch = item.client_id?.toLowerCase().includes(query);
        const userMatch = item.username?.toLowerCase().includes(query);
        const nameMatch = item.device_name?.toLowerCase().includes(query);
        const ipMatch = item.ip_address?.toLowerCase().includes(query);
        return idMatch || userMatch || nameMatch || ipMatch;
      });
    }
    return result;
  }, [records, searchQuery]);

  const formatProtocol = (ver?: number) => {
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

  const formatDate = (dateStr?: string) => {
    if (!dateStr) return '-';
    try {
      const d = new Date(dateStr);
      if (isNaN(d.getTime())) return dateStr;
      return d.toLocaleString('zh-CN', { hour12: false });
    } catch {
      return dateStr;
    }
  };

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto">
      {/* 头部导航区域 */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2.5">
            <History className="w-6 h-6 text-primary" />
            {t('CompilerRecords.title') || '编译机记录'}
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            {t('CompilerRecords.description') ||
              '统一维护所有历史固件编译设备档案及连接状态（每个账号对应唯一独立记录）'}
          </p>
        </div>

        <div className="flex items-center gap-2.5">
          <Button
            className="gap-1.5 shadow-sm"
            onClick={() => {
              setAddError(null);
              setShowAddModal(true);
            }}
          >
            <Plus className="w-4 h-4" />
            {t('MqttClients.addCompiler') || '新建编译机'}
          </Button>
        </div>
      </div>

      {/* 筛选与搜索工具栏 */}
      <div className="flex flex-wrap items-center justify-between gap-3 p-4 rounded-xl border border-border bg-card shadow-xs">
        <div className="flex flex-wrap items-center gap-3">
          {/* 状态筛选 */}
          <div className="w-32">
            <Select
              value={statusFilter}
              onValueChange={(val) => setStatusFilter(val as 'all' | 'online' | 'offline')}
            >
              <SelectTrigger className="h-9">
                <SelectValue placeholder="全部状态" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部状态</SelectItem>
                <SelectItem value="online">在线</SelectItem>
                <SelectItem value="offline">离线</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* 关键字搜索 */}
          <div className="relative w-72">
            <Search className="w-4 h-4 absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder={t('CompilerRecords.searchPlaceholder') || '搜索设备名称 / 用户名 / IP'}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-8 h-9 text-sm"
            />
            {searchQuery && (
              <button
                onClick={() => setSearchQuery('')}
                className="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            )}
          </div>
        </div>

        <Button
          variant="ghost"
          size="sm"
          onClick={() => fetchRecords(true)}
          disabled={loading || refreshing}
          className="gap-1 text-muted-foreground"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${refreshing ? 'animate-spin' : ''}`} />
          {t('CompilerRecords.refresh') || '刷新'}
        </Button>
      </div>

      {/* 列表表格 */}
      {error ? (
        <div className="p-8 rounded-xl border border-border bg-card text-center shadow-xs">
          <div className="w-12 h-12 rounded-full bg-destructive/10 text-destructive flex items-center justify-center mx-auto mb-3">
            <AlertTriangle className="w-6 h-6" />
          </div>
          <h3 className="text-base font-semibold text-foreground">{error}</h3>
          <p className="mt-1.5 text-xs text-muted-foreground">请检查网络或后端数据库状态</p>
          <Button
            variant="outline"
            size="sm"
            onClick={() => fetchRecords(true)}
            className="mt-4"
          >
            重试
          </Button>
        </div>
      ) : (
        <div className="rounded-xl border border-border bg-card shadow-xs overflow-hidden">
          <Table>
            <TableHeader className="bg-muted/40">
              <TableRow>
                <TableHead className="w-24">状态</TableHead>
                <TableHead className="w-44">设备标识 (Client ID)</TableHead>
                <TableHead className="w-36">用户名</TableHead>
                <TableHead className="w-36">设备备注名称</TableHead>
                <TableHead className="w-36">最近连接 IP</TableHead>
                <TableHead className="w-28">协议版本</TableHead>
                <TableHead className="w-40">首次登记时间</TableHead>
                <TableHead className="w-40">最后活跃时间</TableHead>
                <TableHead className="w-28 text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell colSpan={9} className="text-center py-12 text-muted-foreground">
                    <RefreshCw className="w-5 h-5 animate-spin mx-auto mb-2 text-primary" />
                    加载中...
                  </TableCell>
                </TableRow>
              ) : filteredRecords.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={9} className="text-center py-12 text-muted-foreground">
                    {searchQuery
                      ? `没有与 “${searchQuery}” 匹配的编译机记录`
                      : '暂无匹配的编译机档案，请点击右上角新建编译机'}
                  </TableCell>
                </TableRow>
              ) : (
                filteredRecords.map((record) => (
                  <TableRow key={record.id} className="hover:bg-muted/30 transition-colors">
                    {/* 状态 */}
                    <TableCell>
                      {record.online ? (
                        <Badge
                          variant="outline"
                          className="border-green-500 text-green-600 bg-green-50 dark:bg-green-950/40"
                        >
                          正常
                        </Badge>
                      ) : (
                        <Badge variant="secondary">已下线</Badge>
                      )}
                    </TableCell>

                    {/* Client ID */}
                    <TableCell className="font-mono text-xs font-semibold">
                      <div className="flex items-center gap-1.5 group">
                        <span className="truncate max-w-[170px]" title={record.client_id}>
                          {record.client_id}
                        </span>
                        <button
                          onClick={() => handleCopy(record.client_id)}
                          className="text-muted-foreground hover:text-foreground opacity-0 group-hover:opacity-100 transition-opacity"
                          title="复制 Client ID"
                        >
                          {copiedText === record.client_id ? (
                            <Check className="w-3.5 h-3.5 text-green-600" />
                          ) : (
                            <Copy className="w-3.5 h-3.5" />
                          )}
                        </button>
                      </div>
                    </TableCell>

                    {/* Username */}
                    <TableCell className="text-sm font-medium">{record.username}</TableCell>

                    {/* Device Name */}
                    <TableCell>
                      <Badge variant="secondary" className="font-normal">
                        {record.device_name || record.client_id}
                      </Badge>
                    </TableCell>

                    {/* IP & Port */}
                    <TableCell className="text-xs font-mono text-muted-foreground">
                      {record.ip_address ? (
                        <span>
                          {record.ip_address}
                          {record.port ? `:${record.port}` : ''}
                        </span>
                      ) : (
                        '-'
                      )}
                    </TableCell>

                    {/* Protocol */}
                    <TableCell>
                      <Badge variant="secondary" className="font-normal font-mono text-[11px]">
                        {formatProtocol(record.proto_ver)}
                      </Badge>
                    </TableCell>

                    {/* Created At */}
                    <TableCell className="text-xs text-muted-foreground">
                      {formatDate(record.created_at)}
                    </TableCell>

                    {/* Last Active */}
                    <TableCell className="text-xs text-muted-foreground">
                      {record.online
                        ? formatDate(record.connected_at)
                        : formatDate(record.disconnected_at || record.updated_at)}
                    </TableCell>

                    {/* Action */}
                    <TableCell className="text-right">
                      <Button
                        variant="outline"
                        size="sm"
                        className="h-8 gap-1 text-xs"
                        onClick={() => setDetailRecord(record)}
                      >
                        <Info className="w-3 h-3 text-primary" />
                        详情
                      </Button>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </div>
      )}

      {/* 详情弹窗 */}
      {detailRecord && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-xs animate-in fade-in duration-200">
          <div className="bg-card text-card-foreground rounded-xl border border-border shadow-lg w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden">
            <div className="px-6 py-4 border-b border-border flex items-center justify-between">
              <div className="flex items-center gap-2.5">
                <History className="w-5 h-5 text-primary" />
                <div>
                  <h2 className="text-base font-semibold">
                    {t('CompilerRecords.recordDetails') || '编译机档案详情'}
                  </h2>
                  <p className="text-xs text-muted-foreground font-mono truncate max-w-sm">
                    {detailRecord.client_id}
                  </p>
                </div>
              </div>
              <button
                onClick={() => setDetailRecord(null)}
                className="p-1 rounded-md text-muted-foreground hover:text-foreground"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="p-6 overflow-y-auto space-y-4 text-xs">
              <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
                <div className="p-3 rounded-lg border border-border bg-muted/20">
                  <div className="text-muted-foreground mb-1">连接状态</div>
                  <div className="font-semibold">
                    {detailRecord.online ? (
                      <span className="text-green-600 flex items-center gap-1">正常 (在线)</span>
                    ) : (
                      <span className="text-muted-foreground flex items-center gap-1">已下线</span>
                    )}
                  </div>
                </div>

                <div className="p-3 rounded-lg border border-border bg-muted/20">
                  <div className="text-muted-foreground mb-1">MQTT 账号</div>
                  <div className="font-medium truncate">{detailRecord.username}</div>
                </div>

                <div className="p-3 rounded-lg border border-border bg-muted/20">
                  <div className="text-muted-foreground mb-1">设备显示名称</div>
                  <div className="font-medium truncate">
                    {detailRecord.device_name || detailRecord.client_id}
                  </div>
                </div>

                <div className="p-3 rounded-lg border border-border bg-muted/20">
                  <div className="text-muted-foreground mb-1">协议版本</div>
                  <div className="font-medium font-mono">{formatProtocol(detailRecord.proto_ver)}</div>
                </div>

                <div className="p-3 rounded-lg border border-border bg-muted/20">
                  <div className="text-muted-foreground mb-1">IP 与端口</div>
                  <div className="font-medium font-mono">
                    {detailRecord.ip_address
                      ? `${detailRecord.ip_address}:${detailRecord.port || ''}`
                      : '-'}
                  </div>
                </div>

                <div className="p-3 rounded-lg border border-border bg-muted/20">
                  <div className="text-muted-foreground mb-1">心跳间隔</div>
                  <div className="font-medium font-mono">
                    {detailRecord.keepalive !== undefined ? `${detailRecord.keepalive}s` : '-'}
                  </div>
                </div>
              </div>

              <div>
                <h3 className="text-xs font-semibold uppercase tracking-wider mb-2 text-muted-foreground">
                  全生命周期时间戳
                </h3>
                <div className="p-3.5 rounded-lg border border-border bg-muted/20 space-y-2 font-mono">
                  <div className="flex justify-between py-1 border-b border-border/50">
                    <span className="text-muted-foreground font-sans">首次登记时间</span>
                    <span>{formatDate(detailRecord.created_at)}</span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-border/50">
                    <span className="text-muted-foreground font-sans">最后上线时间</span>
                    <span>{formatDate(detailRecord.connected_at)}</span>
                  </div>
                  <div className="flex justify-between py-1 border-b border-border/50">
                    <span className="text-muted-foreground font-sans">最后离线时间</span>
                    <span>{formatDate(detailRecord.disconnected_at)}</span>
                  </div>
                  <div className="flex justify-between py-1">
                    <span className="text-muted-foreground font-sans">离线原因</span>
                    <span>{detailRecord.disconnected_reason || '-'}</span>
                  </div>
                </div>
              </div>
            </div>

            <div className="px-6 py-3.5 border-t border-border flex justify-end">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setDetailRecord(null)}
                className="h-8 text-xs"
              >
                关闭
              </Button>
            </div>
          </div>
        </div>
      )}

      {/* 新建编译机弹窗 */}
      {showAddModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-background/80 backdrop-blur-xs animate-in fade-in duration-200">
          <div className="bg-card text-card-foreground rounded-xl border border-border shadow-lg w-full max-w-lg overflow-hidden flex flex-col max-h-[90vh]">
            <div className="px-6 py-4 border-b border-border flex items-center justify-between">
              <div className="flex items-center gap-2.5">
                <Plus className="w-5 h-5 text-primary" />
                <div>
                  <h2 className="text-base font-semibold">
                    {t('MqttClients.addCompilerTitle') || '新建固件编译机'}
                  </h2>
                  <p className="text-xs text-muted-foreground">
                    {t('MqttClients.addCompilerDesc') || '录入并登记编译机信息至数据库（每个账号保存一条唯一档案）'}
                  </p>
                </div>
              </div>
              <button
                onClick={() => setShowAddModal(false)}
                className="p-1 rounded-md text-muted-foreground hover:text-foreground"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <form onSubmit={handleAddCompiler} className="flex flex-col flex-1 overflow-hidden">
              <div className="p-6 overflow-y-auto space-y-4 text-xs">
                {addError && (
                  <div className="p-3 rounded-lg bg-destructive/10 border border-destructive/20 text-destructive flex items-start gap-2">
                    <AlertTriangle className="w-4 h-4 flex-shrink-0 mt-0.5" />
                    <span>{addError}</span>
                  </div>
                )}

                <div>
                  <label className="block font-medium mb-1.5">
                    MQTT 账号 (Username) <span className="text-destructive">*</span>
                  </label>
                  <Input
                    type="text"
                    required
                    placeholder="如：compiler_node_01"
                    value={addForm.username}
                    onChange={(e) => {
                      const val = e.target.value;
                      setAddForm((prev) => ({
                        ...prev,
                        username: val,
                        clientId: prev.clientId === '' || prev.clientId === `compiler-${prev.username}` ? `compiler-${val}` : prev.clientId,
                      }));
                    }}
                    className="h-9 text-xs"
                  />
                  <p className="mt-1 text-[11px] text-muted-foreground">
                    编译机连接 MQTT Broker 时的认证账号（每个账号在数据库中唯一）
                  </p>
                </div>

                <div>
                  <label className="block font-medium mb-1.5">
                    设备标识 (Client ID) <span className="text-destructive">*</span>
                  </label>
                  <Input
                    type="text"
                    required
                    placeholder="如：compiler-01"
                    value={addForm.clientId}
                    onChange={(e) => setAddForm((prev) => ({ ...prev, clientId: e.target.value }))}
                    className="h-9 text-xs font-mono"
                  />
                  <p className="mt-1 text-[11px] text-muted-foreground">
                    设备端 MQTT 客户端唯一标识符
                  </p>
                </div>

                <div>
                  <label className="block font-medium mb-1.5">
                    设备显示名称 (选填)
                  </label>
                  <Input
                    type="text"
                    placeholder="如：华南研发部-1号编译机 (留空则默认同 Client ID)"
                    value={addForm.deviceName}
                    onChange={(e) => setAddForm((prev) => ({ ...prev, deviceName: e.target.value }))}
                    className="h-9 text-xs"
                  />
                </div>

                <div className="grid grid-cols-3 gap-3">
                  <div className="col-span-2">
                    <label className="block font-medium mb-1.5">
                      IP 地址 (选填)
                    </label>
                    <Input
                      type="text"
                      placeholder="如：192.168.1.100"
                      value={addForm.ipAddress}
                      onChange={(e) => setAddForm((prev) => ({ ...prev, ipAddress: e.target.value }))}
                      className="h-9 text-xs font-mono"
                    />
                  </div>
                  <div>
                    <label className="block font-medium mb-1.5">
                      端口 (选填)
                    </label>
                    <Input
                      type="number"
                      placeholder="1883"
                      value={addForm.port}
                      onChange={(e) => setAddForm((prev) => ({ ...prev, port: e.target.value }))}
                      className="h-9 text-xs font-mono"
                    />
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label className="block font-medium mb-1.5">
                      协议版本
                    </label>
                    <Select
                      value={addForm.protoVer}
                      onValueChange={(val) => setAddForm((prev) => ({ ...prev, protoVer: val }))}
                    >
                      <SelectTrigger className="h-9">
                        <SelectValue placeholder="协议版本" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="5">MQTT 5.0 (推荐)</SelectItem>
                        <SelectItem value="4">MQTT 3.1.1</SelectItem>
                        <SelectItem value="3">MQTT 3.1</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div>
                    <label className="block font-medium mb-1.5">
                      心跳间隔 (秒)
                    </label>
                    <Input
                      type="number"
                      placeholder="60"
                      value={addForm.keepalive}
                      onChange={(e) => setAddForm((prev) => ({ ...prev, keepalive: e.target.value }))}
                      className="h-9 text-xs font-mono"
                    />
                  </div>
                </div>

                <div className="pt-2 border-t border-border flex items-center justify-between">
                  <div>
                    <div className="font-medium">初始状态</div>
                    <div className="text-[11px] text-muted-foreground">
                      {addForm.online ? '直接标记为正常 (在线)' : '标记为已下线 (等待设备连接)'}
                    </div>
                  </div>
                  <Button
                    type="button"
                    variant={addForm.online ? 'default' : 'outline'}
                    size="sm"
                    onClick={() => setAddForm((prev) => ({ ...prev, online: !prev.online }))}
                    className="h-7 text-xs"
                  >
                    {addForm.online ? '在线' : '离线'}
                  </Button>
                </div>
              </div>

              <div className="px-6 py-3.5 border-t border-border flex justify-end gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => setShowAddModal(false)}
                  disabled={adding}
                  className="h-8 text-xs"
                >
                  取消
                </Button>
                <Button
                  type="submit"
                  size="sm"
                  disabled={adding}
                  className="h-8 text-xs gap-1.5 shadow-sm"
                >
                  {adding && <RefreshCw className="w-3.5 h-3.5 animate-spin" />}
                  <span>{adding ? '正在保存...' : '确认新建'}</span>
                </Button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
}

export default CompilerRecordsPage;
