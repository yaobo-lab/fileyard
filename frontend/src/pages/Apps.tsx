import { useState, useEffect, useMemo } from 'react';
import { useAuthFetch } from '@/context/AuthContext';
import { useModalDialog } from '@/context/ModalDialogContext';
import { createAppService } from '@/services/appService';
import { AppItem, AppClass } from '@/types/app';
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
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Boxes,
  Plus,
  Search,
  RefreshCw,
  Layers,
  MoreHorizontal,
  Server,
  Users,
  Play,
  Pencil,
  Trash2,
  FileText,
  GitFork,
  ExternalLink,
} from 'lucide-react';
import { CreateAppModal } from '@/components/apps/CreateAppModal';
import { AppBasicModal } from '@/components/apps/AppBasicModal';
import { AppDeployModal } from '@/components/apps/AppDeployModal';
import { AppMembersModal } from '@/components/apps/AppMembersModal';
import { AppClassModal } from '@/components/apps/AppClassModal';
import { AppPipelineModal } from '@/components/apps/AppPipelineModal';

export function AppsPage() {
  const authFetch = useAuthFetch();
  const { confirm: modalConfirm, alert: modalAlert } = useModalDialog();
  const appService = useMemo(() => createAppService(authFetch), [authFetch]);

  const [apps, setApps] = useState<AppItem[]>([]);
  const [total, setTotal] = useState(0);
  const [classList, setClassList] = useState<AppClass[]>([]);
  const [loading, setLoading] = useState(false);

  // 筛选参数
  const [page, setPage] = useState(1);
  const [pageSize] = useState(10);
  const [statusFilter, setStatusFilter] = useState<number>(-1);
  const [classFilter, setClassFilter] = useState<string>('all');
  const [search, setSearch] = useState('');

  // 弹窗状态
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showClassModal, setShowClassModal] = useState(false);
  const [showBasicModal, setShowBasicModal] = useState(false);
  const [showDeployModal, setShowDeployModal] = useState(false);
  const [showMembersModal, setShowMembersModal] = useState(false);
  const [showPipelineModal, setShowPipelineModal] = useState(false);
  const [selectedApp, setSelectedApp] = useState<AppItem | null>(null);

  const loadClasses = async () => {
    try {
      const res = await appService.getClassList();
      setClassList(res.items);
    } catch (err) {
      console.error('Failed to load class list', err);
    }
  };

  const loadApps = async () => {
    setLoading(true);
    try {
      const res = await appService.getAppList({
        page,
        limit: pageSize,
        status: statusFilter,
        class_no: classFilter === 'all' ? undefined : classFilter,
        search: search.trim() || undefined,
      });
      setApps(res.items);
      setTotal(res.total);
    } catch (err) {
      console.error('Failed to load apps', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadClasses();
  }, []);

  useEffect(() => {
    loadApps();
  }, [page, statusFilter, classFilter]);

  const handleSearchSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setPage(1);
    loadApps();
  };

  const handleCreateApp = async (data: any) => {
    await appService.createApp(data);
    loadApps();
  };

  const handleSaveBasic = async (data: any) => {
    if (!selectedApp) return;
    await appService.saveAppBasic(selectedApp.number, data);
    loadApps();
  };

  const handleDeleteApp = async (app: AppItem) => {
    const ok = await modalConfirm({
      title: '确认删除固件',
      description: `确定要删除固件 "${app.name}" (${app.number}) 吗？此操作将软删除该固件。`,
      variant: 'destructive',
      confirmText: '确认删除',
      cancelText: '取消',
    });
    if (!ok) return;

    try {
      await appService.deleteApp(app.number);
      loadApps();
    } catch (err: any) {
      await modalAlert({
        title: '删除失败',
        description: err.message || '删除失败，请稍后重试',
        variant: 'destructive',
      });
    }
  };

  const totalPages = Math.ceil(total / pageSize) || 1;

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto">
      {/* 头部导航区域 */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2.5">
            <Boxes className="w-6 h-6 text-primary" />
            固件管理
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            统一维护固件、编译环境、流水线
          </p>
        </div>

        <div className="flex items-center gap-2.5">
          <Button
            variant="outline"
            className="gap-1.5"
            onClick={() => setShowClassModal(true)}
          >
            <Layers className="w-4 h-4 text-muted-foreground" />
            分类管理
          </Button>
          <Button className="gap-1.5 shadow-sm" onClick={() => setShowCreateModal(true)}>
            <Plus className="w-4 h-4" />
            新建固件
          </Button>
        </div>
      </div>

      {/* 筛选与搜索工具栏 */}
      <div className="flex flex-wrap items-center justify-between gap-3 p-4 rounded-xl border border-border bg-card shadow-xs">
        <div className="flex flex-wrap items-center gap-3">
          {/* 状态筛选 */}
          <div className="w-32">
            <Select
              value={statusFilter.toString()}
              onValueChange={(val) => {
                setStatusFilter(parseInt(val, 10));
                setPage(1);
              }}
            >
              <SelectTrigger className="h-9">
                <SelectValue placeholder="固件状态" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="-1">全部状态</SelectItem>
                <SelectItem value="2">正常</SelectItem>
                <SelectItem value="1">已下线</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* 分类筛选 */}
          <div className="w-40">
            <Select
              value={classFilter}
              onValueChange={(val) => {
                setClassFilter(val);
                setPage(1);
              }}
            >
              <SelectTrigger className="h-9">
                <SelectValue placeholder="所属分类" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部分类</SelectItem>
                {classList.map((c) => (
                  <SelectItem key={c.number} value={c.number}>
                    {c.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* 关键字搜索 */}
          <form onSubmit={handleSearchSubmit} className="relative w-64">
            <Search className="w-4 h-4 absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
            <Input
              placeholder="搜索固件名称 / 编号"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-8 h-9 text-sm"
            />
          </form>
        </div>

        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            setPage(1);
            loadApps();
          }}
          disabled={loading}
          className="gap-1 text-muted-foreground"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
          刷新
        </Button>
      </div>

      {/* 固件列表表格 */}
      <div className="rounded-xl border border-border bg-card shadow-xs overflow-hidden">
        <Table>
          <TableHeader className="bg-muted/40">
            <TableRow>
              <TableHead className="w-20">状态</TableHead>
              <TableHead className="w-36">编号</TableHead>
              <TableHead className="w-48">固件名称</TableHead>
              <TableHead className="w-28">分类</TableHead>
              <TableHead className="w-28">负责人</TableHead>
              <TableHead className="w-36">代码仓库</TableHead>
              <TableHead className="w-36">最后更新时间</TableHead>
              <TableHead className="w-44 text-right">操作</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              <TableRow>
                <TableCell colSpan={8} className="text-center py-12 text-muted-foreground">
                  <RefreshCw className="w-5 h-5 animate-spin mx-auto mb-2 text-primary" />
                  加载中...
                </TableCell>
              </TableRow>
            ) : apps.length === 0 ? (
              <TableRow>
                <TableCell colSpan={8} className="text-center py-12 text-muted-foreground">
                  暂无匹配的固件数据，请点击右上角新建固件
                </TableCell>
              </TableRow>
            ) : (
              apps.map((app) => (
                <TableRow key={app.id} className="hover:bg-muted/30 transition-colors">
                  <TableCell>
                    {app.status === 2 ? (
                      <Badge variant="outline" className="border-green-500 text-green-600 bg-green-50 dark:bg-green-950/40">
                        正常
                      </Badge>
                    ) : (
                      <Badge variant="secondary">已下线</Badge>
                    )}
                  </TableCell>
                  <TableCell className="font-mono text-xs font-semibold">{app.number}</TableCell>
                  <TableCell className="font-medium text-foreground">
                    {app.name}
                  </TableCell>
                  <TableCell>
                    <Badge variant="secondary" className="font-normal">
                      {app.class_name || app.class_no || '未分类'}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-sm">{app.createby_name || '-'}</TableCell>
                  <TableCell>
                    {app.gitlab_id ? (
                      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                        <GitFork className="w-3.5 h-3.5 text-primary" />
                        <span>ID: {app.gitlab_id}</span>
                        {app.git_url && (
                          <a
                            href={app.git_url}
                            target="_blank"
                            rel="noreferrer"
                            className="text-primary hover:underline ml-1"
                          >
                            <ExternalLink className="w-3 h-3" />
                          </a>
                        )}
                      </div>
                    ) : (
                      <span className="text-xs text-muted-foreground/60">未绑定</span>
                    )}
                  </TableCell>
                  <TableCell className="text-xs text-muted-foreground">
                    {app.lastupdate_time
                      ? new Date(app.lastupdate_time).toLocaleString('zh-CN', { hour12: false })
                      : '-'}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-1.5">
                      {/* 流水线快捷按钮 */}
                      <Button
                        variant="outline"
                        size="sm"
                        className="h-8 gap-1 text-xs"
                        onClick={() => {
                          setSelectedApp(app);
                          setShowPipelineModal(true);
                        }}
                      >
                        <Play className="w-3 h-3 fill-current text-primary" />
                        流水线
                      </Button>

                      {/* 更多操作下拉菜单 */}
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                            <MoreHorizontal className="w-4 h-4" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end" className="w-40">
                          <DropdownMenuItem
                            className="gap-2 text-xs cursor-pointer"
                            onClick={() => {
                              setSelectedApp(app);
                              setShowDeployModal(true);
                            }}
                          >
                            <Server className="w-3.5 h-3.5" />
                            编译环境配置
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            className="gap-2 text-xs cursor-pointer"
                            onClick={() => {
                              setSelectedApp(app);
                              setShowMembersModal(true);
                            }}
                          >
                            <Users className="w-3.5 h-3.5" />
                            参与人员管理
                          </DropdownMenuItem>
                          <DropdownMenuItem
                            className="gap-2 text-xs cursor-pointer"
                            onClick={() => {
                              setSelectedApp(app);
                              setShowBasicModal(true);
                            }}
                          >
                            <Pencil className="w-3.5 h-3.5" />
                            编辑基础信息
                          </DropdownMenuItem>
                          {app.doc_path && (
                            <DropdownMenuItem asChild className="gap-2 text-xs cursor-pointer">
                              <a href={app.doc_path} target="_blank" rel="noreferrer">
                                <FileText className="w-3.5 h-3.5" />
                                查看文档
                              </a>
                            </DropdownMenuItem>
                          )}
                          <DropdownMenuSeparator />
                          <DropdownMenuItem
                            className="gap-2 text-xs text-destructive focus:text-destructive cursor-pointer"
                            onClick={() => handleDeleteApp(app)}
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                            删除固件
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </div>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>

        {/* 分页控制器 */}
        {total > pageSize && (
          <div className="flex items-center justify-between px-6 py-4 border-t border-border bg-muted/10">
            <div className="text-xs text-muted-foreground">
              共 <span className="font-medium text-foreground">{total}</span> 条固件，当前第 {page} / {totalPages} 页
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

      {/* 弹窗集合 */}
      <CreateAppModal
        open={showCreateModal}
        onOpenChange={setShowCreateModal}
        classList={classList}
        onSubmit={handleCreateApp}
      />

      <AppClassModal
        open={showClassModal}
        onOpenChange={setShowClassModal}
        classList={classList}
        onRefresh={loadClasses}
        appService={appService}
      />

      <AppBasicModal
        open={showBasicModal}
        onOpenChange={setShowBasicModal}
        app={selectedApp}
        classList={classList}
        onSubmit={handleSaveBasic}
      />

      <AppDeployModal
        open={showDeployModal}
        onOpenChange={setShowDeployModal}
        app={selectedApp}
        appService={appService}
      />

      <AppMembersModal
        open={showMembersModal}
        onOpenChange={setShowMembersModal}
        app={selectedApp}
        appService={appService}
      />

      <AppPipelineModal
        open={showPipelineModal}
        onOpenChange={setShowPipelineModal}
        app={selectedApp}
        appService={appService}
      />
    </div>
  );
}
