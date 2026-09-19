import { useState, useEffect, useMemo } from 'react';
import { useAuthFetch } from '@/context/AuthContext';
import { createConfigService } from '@/services/configService';
import { AppConfig } from '@/types/app';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Sliders,
  Plus,
  Search,
  RefreshCw,
  Pencil,
  Trash2,
  FileCode,
} from 'lucide-react';
import { ConfigEditModal } from '@/components/configs/ConfigEditModal';

export function ConfigsPage() {
  const authFetch = useAuthFetch();
  const configService = useMemo(() => createConfigService(authFetch), [authFetch]);

  const [configs, setConfigs] = useState<AppConfig[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);

  const [page, setPage] = useState(1);
  const [pageSize] = useState(10);
  const [search, setSearch] = useState('');

  const [showEditModal, setShowEditModal] = useState(false);
  const [selectedConfig, setSelectedConfig] = useState<AppConfig | null>(null);

  const loadConfigs = async () => {
    setLoading(true);
    try {
      const res = await configService.getConfigList({
        page,
        limit: pageSize,
        search: search.trim() || undefined,
      });
      setConfigs(res.items);
      setTotal(res.total);
    } catch (err) {
      console.error('Failed to load configs', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadConfigs();
  }, [page]);

  const handleSearchSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setPage(1);
    loadConfigs();
  };

  const handleSaveConfig = async (data: Partial<AppConfig>) => {
    await configService.saveConfig(data);
    loadConfigs();
  };

  const handleDeleteConfig = async (cfg: AppConfig) => {
    if (!confirm(`确定要删除配置 "${cfg.name}" (${cfg.key}) 吗？`)) return;
    try {
      await configService.deleteConfig(cfg.id);
      loadConfigs();
    } catch (err: any) {
      alert(err.message || '删除失败');
    }
  };

  const totalPages = Math.ceil(total / pageSize) || 1;

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto">
      {/* 头部区域 */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2.5">
            <Sliders className="w-6 h-6 text-primary" />
            配置管理
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            集中管理全局通用配置、GitLab CI/CD 模板、集群与环境参数
          </p>
        </div>

        <Button
          className="gap-1.5 shadow-sm"
          onClick={() => {
            setSelectedConfig(null);
            setShowEditModal(true);
          }}
        >
          <Plus className="w-4 h-4" />
          新建配置
        </Button>
      </div>

      {/* 搜索过滤工具栏 */}
      <div className="flex flex-wrap items-center justify-between gap-3 p-4 rounded-xl border border-border bg-card shadow-xs">
        <form onSubmit={handleSearchSubmit} className="relative w-72">
          <Search className="w-4 h-4 absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="搜索配置名称 / Key / 编号"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-8 h-9 text-sm"
          />
        </form>

        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setPage(1);
            loadConfigs();
          }}
          disabled={loading}
          className="gap-1 text-muted-foreground"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
          刷新
        </Button>
      </div>

      {/* 配置列表表格 */}
      <div className="rounded-xl border border-border bg-card shadow-xs overflow-hidden">
        <Table>
          <TableHeader className="bg-muted/40">
            <TableRow>
              <TableHead className="w-36">配置编号</TableHead>
              <TableHead className="w-48">配置名称</TableHead>
              <TableHead className="w-48">配置 Key</TableHead>
              <TableHead>备注说明</TableHead>
              <TableHead className="w-40">创建时间</TableHead>
              <TableHead className="w-28 text-right">操作</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center py-12 text-muted-foreground">
                  <RefreshCw className="w-5 h-5 animate-spin mx-auto mb-2 text-primary" />
                  加载中...
                </TableCell>
              </TableRow>
            ) : configs.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center py-12 text-muted-foreground">
                  暂无匹配的配置项，请点击右上角新建配置
                </TableCell>
              </TableRow>
            ) : (
              configs.map((cfg) => (
                <TableRow key={cfg.id} className="hover:bg-muted/30 transition-colors">
                  <TableCell className="font-mono text-xs font-semibold">{cfg.number}</TableCell>
                  <TableCell>
                    <div className="font-medium text-foreground flex items-center gap-1.5">
                      <FileCode className="w-3.5 h-3.5 text-primary" />
                      {cfg.name}
                    </div>
                  </TableCell>
                  <TableCell className="font-mono text-xs text-muted-foreground">
                    {cfg.key}
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {cfg.remark || '-'}
                  </TableCell>
                  <TableCell className="text-xs text-muted-foreground">
                    {cfg.create_time
                      ? new Date(cfg.create_time).toLocaleString('zh-CN', { hour12: false })
                      : '-'}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-8 w-8 p-0 text-muted-foreground hover:text-foreground"
                        onClick={() => {
                          setSelectedConfig(cfg);
                          setShowEditModal(true);
                        }}
                      >
                        <Pencil className="w-3.5 h-3.5" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-8 w-8 p-0 text-destructive hover:text-destructive hover:bg-destructive/10"
                        onClick={() => handleDeleteConfig(cfg)}
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>

        {/* 分页 */}
        {total > pageSize && (
          <div className="flex items-center justify-between px-6 py-4 border-t border-border bg-muted/10">
            <div className="text-xs text-muted-foreground">
              共 <span className="font-medium text-foreground">{total}</span> 项配置，当前第 {page} / {totalPages} 页
            </div>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPage((p) => Math.max(1, p - 1))}
                disabled={page <= 1}
              >
                上一页
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                disabled={page >= totalPages}
              >
                下一页
              </Button>
            </div>
          </div>
        )}
      </div>

      <ConfigEditModal
        open={showEditModal}
        onOpenChange={setShowEditModal}
        config={selectedConfig}
        onSubmit={handleSaveConfig}
      />
    </div>
  );
}
