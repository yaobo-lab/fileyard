import { useState, useEffect } from 'react';
import { AppItem, AppDeploy, GitlabPipeline } from '@/types/app';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Play,
  FileCode,
  RefreshCw,
  ExternalLink,
  CheckCircle,
  XCircle,
  Clock,
  Loader2,
  AlertCircle,
  GitBranch,
} from 'lucide-react';
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

interface AppPipelineModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  app: AppItem | null;
  appService: any;
}

export function AppPipelineModal({
  open,
  onOpenChange,
  app,
  appService,
}: AppPipelineModalProps) {
  const [deploys, setDeploys] = useState<AppDeploy[]>([]);
  const [selectedDeployNo, setSelectedDeployNo] = useState<string>('');
  const [pipelines, setPipelines] = useState<GitlabPipeline[]>([]);
  const [loading, setLoading] = useState(false);
  const [triggering, setTriggering] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [ciContent, setCiContent] = useState<string | null>(null);
  const [showCiModal, setShowCiModal] = useState(false);
  const [loadingCi, setLoadingCi] = useState(false);
  const [message, setMessage] = useState<{ text: string; isError: boolean } | null>(null);

  const loadData = async () => {
    if (!app) return;
    setLoading(true);
    setMessage(null);
    try {
      const [deployList, pipelineList] = await Promise.all([
        appService.getDeployList(app.number),
        app.gitlab_id ? appService.getPipelineHistory(app.number).catch(() => []) : Promise.resolve([]),
      ]);
      setDeploys(deployList);
      if (deployList.length > 0 && !selectedDeployNo) {
        setSelectedDeployNo(deployList[0].number);
      }
      setPipelines(pipelineList);
    } catch (err: any) {
      setMessage({ text: err.message || '加载流水线数据失败', isError: true });
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (open && app) {
      loadData();
    }
  }, [open, app]);

  const handleTrigger = async () => {
    if (!app || !selectedDeployNo) return;
    setTriggering(true);
    setMessage(null);
    try {
      await appService.triggerCi(app.number, selectedDeployNo);
      setMessage({ text: '流水线已成功触发！', isError: false });
      await loadData();
    } catch (err: any) {
      setMessage({ text: err.message || '触发流水线失败', isError: true });
    } finally {
      setTriggering(false);
    }
  };

  const handleViewCi = async () => {
    if (!app) return;
    setLoadingCi(true);
    setMessage(null);
    try {
      const content = await appService.getCiFile(app.number, selectedDeployNo);
      setCiContent(content || '# 暂无 .gitlab-ci.yml 配置文件内容');
      setShowCiModal(true);
    } catch (err: any) {
      setMessage({ text: err.message || '获取 CI 文件失败', isError: true });
    } finally {
      setLoadingCi(false);
    }
  };

  const handleSyncCi = async () => {
    if (!app) return;
    setSyncing(true);
    setMessage(null);
    try {
      await appService.syncCi(app.number);
      setMessage({ text: 'CI 模板已同步完成', isError: false });
    } catch (err: any) {
      setMessage({ text: err.message || '同步 CI 模板失败', isError: true });
    } finally {
      setSyncing(false);
    }
  };

  const renderStatus = (status: string) => {
    switch (status) {
      case 'success':
        return (
          <Badge variant="outline" className="gap-1 border-green-500 text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-950/40">
            <CheckCircle className="w-3 h-3" />
            成功
          </Badge>
        );
      case 'failed':
        return (
          <Badge variant="destructive" className="gap-1">
            <XCircle className="w-3 h-3" />
            失败
          </Badge>
        );
      case 'running':
        return (
          <Badge variant="outline" className="gap-1 border-blue-500 text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-950/40">
            <Loader2 className="w-3 h-3 animate-spin" />
            运行中
          </Badge>
        );
      case 'pending':
        return (
          <Badge variant="secondary" className="gap-1">
            <Clock className="w-3 h-3" />
            排队中
          </Badge>
        );
      case 'canceled':
        return <Badge variant="secondary">已取消</Badge>;
      case 'skipped':
        return <Badge variant="secondary">已跳过</Badge>;
      case 'manual':
        return <Badge variant="outline">等待人工</Badge>;
      default:
        return <Badge variant="secondary">{status}</Badge>;
    }
  };

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="sm:max-w-[840px] max-h-[85vh] flex flex-col p-0 overflow-hidden">
          <DialogHeader className="px-6 pt-6 pb-4 border-b border-border">
            <DialogTitle className="text-xl font-semibold flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Play className="w-5 h-5 text-primary" />
                CI/CD 流水线管理
                {app && <span className="text-sm font-normal text-muted-foreground">({app.name})</span>}
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={loadData}
                disabled={loading}
                className="gap-1 text-xs"
              >
                <RefreshCw className={`w-3.5 h-3.5 ${loading ? 'animate-spin' : ''}`} />
                刷新
              </Button>
            </DialogTitle>
          </DialogHeader>

          <div className="p-6 overflow-y-auto space-y-6">
            {message && (
              <div
                className={`p-3 text-sm rounded-md border flex items-center gap-2 ${message.isError
                  ? 'text-red-600 bg-red-50 dark:bg-red-950/40 border-red-200'
                  : 'text-green-600 bg-green-50 dark:bg-green-950/40 border-green-200'
                  }`}
              >
                {message.isError ? <AlertCircle className="w-4 h-4 shrink-0" /> : <CheckCircle className="w-4 h-4 shrink-0" />}
                {message.text}
              </div>
            )}

            {/* 控制面板 */}
            <div className="flex flex-wrap items-center justify-between gap-4 p-4 rounded-xl border border-border bg-muted/20">
              <div className="flex items-center gap-3">
                <div className="text-sm font-medium">编译环境：</div>
                <div className="w-48">
                  <Select value={selectedDeployNo} onValueChange={setSelectedDeployNo}>
                    <SelectTrigger className="h-9">
                      <SelectValue placeholder="选择部署环境" />
                    </SelectTrigger>
                    <SelectContent>
                      {deploys.length === 0 ? (
                        <SelectItem value="none" disabled>暂无配置环境</SelectItem>
                      ) : (
                        deploys.map((d) => (
                          <SelectItem key={d.number} value={d.number}>
                            {d.name} ({d.branch_name || 'master'})
                          </SelectItem>
                        ))
                      )}
                    </SelectContent>
                  </Select>
                </div>
              </div>

              <div className="flex items-center gap-2">
                <Button
                  onClick={handleTrigger}
                  disabled={triggering || deploys.length === 0}
                  className="gap-1.5 shadow-sm"
                >
                  <Play className="w-4 h-4 fill-current" />
                  {triggering ? '启动中...' : '运行流水线'}
                </Button>

                <Button
                  variant="outline"
                  onClick={handleViewCi}
                  disabled={loadingCi}
                  className="gap-1.5"
                >
                  <FileCode className="w-4 h-4" />
                  查看 CI 脚本
                </Button>

                <Button
                  variant="secondary"
                  onClick={handleSyncCi}
                  disabled={syncing}
                  className="gap-1.5"
                >
                  <RefreshCw className={`w-3.5 h-3.5 ${syncing ? 'animate-spin' : ''}`} />
                  同步CI模板
                </Button>
              </div>
            </div>

            {/* 流水线构建历史表格 */}
            <div>
              <div className="text-sm font-semibold mb-3 flex items-center justify-between">
                <span>构建历史记录</span>
                <span className="text-xs font-normal text-muted-foreground">共 {pipelines.length} 条流水线记录</span>
              </div>

              <div className="rounded-lg border border-border overflow-hidden">
                <Table>
                  <TableHeader className="bg-muted/40">
                    <TableRow>
                      <TableHead className="w-20">ID</TableHead>
                      <TableHead className="w-40">关联分支</TableHead>
                      <TableHead className="w-28">状态</TableHead>
                      <TableHead>触发时间</TableHead>
                      <TableHead className="w-28 text-right">GitLab</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {loading ? (
                      <TableRow>
                        <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                          <Loader2 className="w-5 h-5 animate-spin mx-auto mb-2 text-primary" />
                          正在拉取流水线记录...
                        </TableCell>
                      </TableRow>
                    ) : pipelines.length === 0 ? (
                      <TableRow>
                        <TableCell colSpan={5} className="text-center py-8 text-muted-foreground">
                          {app?.gitlab_id ? '暂无流水线构建记录，请点击上方“运行流水线”启动' : '请先在固件基本信息中配置 GitLab 项目 ID'}
                        </TableCell>
                      </TableRow>
                    ) : (
                      pipelines.map((p) => (
                        <TableRow key={p.id}>
                          <TableCell className="font-mono text-xs font-semibold">#{p.id}</TableCell>
                          <TableCell>
                            <div className="flex items-center gap-1.5 font-mono text-xs">
                              <GitBranch className="w-3.5 h-3.5 text-muted-foreground" />
                              {p.ref}
                            </div>
                          </TableCell>
                          <TableCell>{renderStatus(p.status)}</TableCell>
                          <TableCell className="text-xs text-muted-foreground">
                            {new Date(p.created_at).toLocaleString('zh-CN', { hour12: false })}
                          </TableCell>
                          <TableCell className="text-right">
                            {p.web_url ? (
                              <Button
                                variant="ghost"
                                size="sm"
                                asChild
                                className="h-7 gap-1 text-xs text-primary hover:text-primary"
                              >
                                <a href={p.web_url} target="_blank" rel="noreferrer">
                                  打开
                                  <ExternalLink className="w-3 h-3" />
                                </a>
                              </Button>
                            ) : (
                              '-'
                            )}
                          </TableCell>
                        </TableRow>
                      ))
                    )}
                  </TableBody>
                </Table>
              </div>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* 查看 CI 文本弹窗 */}
      <Dialog open={showCiModal} onOpenChange={setShowCiModal}>
        <DialogContent className="sm:max-w-[720px] max-h-[80vh] flex flex-col">
          <DialogHeader>
            <DialogTitle className="text-lg font-semibold flex items-center gap-2">
              <FileCode className="w-5 h-5 text-primary" />
              .gitlab-ci.yml 内容预览
            </DialogTitle>
          </DialogHeader>
          <div className="flex-1 overflow-y-auto p-4 rounded-md bg-zinc-950 text-zinc-100 font-mono text-xs leading-relaxed border border-zinc-800">
            <pre className="whitespace-pre-wrap select-all">{ciContent}</pre>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}
