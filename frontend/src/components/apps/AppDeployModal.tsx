import { useState, useEffect } from 'react';
import { useModalDialog } from '@/context/ModalDialogContext';
import { AppItem, AppDeploy, GitlabBranch } from '@/types/app';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Plus, Copy, Trash2, CheckCircle2, Server, GitBranch } from 'lucide-react';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

interface AppDeployModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  app: AppItem | null;
  appService: any;
}

export function AppDeployModal({
  open,
  onOpenChange,
  app,
  appService,
}: AppDeployModalProps) {
  const { confirm: modalConfirm } = useModalDialog();
  const [deploys, setDeploys] = useState<AppDeploy[]>([]);
  const [branches, setBranches] = useState<GitlabBranch[]>([]);
  const [activeDeploy, setActiveDeploy] = useState<AppDeploy | null>(null);
  const [isNew, setIsNew] = useState(false);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  // 表单状态
  const [name, setName] = useState('');
  const [branchName, setBranchName] = useState('');
  const [buildTag, setBuildTag] = useState('');
  const [autoPub, setAutoPub] = useState(false);
  const [apiUri, setApiUri] = useState('');
  const [apiKeyId, setApiKeyId] = useState<number>(0);
  const [envs, setEnvs] = useState('');

  const loadData = async () => {
    if (!app) return;
    setLoading(true);
    setError('');
    try {
      const [deployList, branchList] = await Promise.all([
        appService.getDeployList(app.number),
        app.gitlab_id ? appService.getBranches(app.number).catch(() => []) : Promise.resolve([]),
      ]);
      setDeploys(deployList);
      setBranches(branchList);
      if (deployList.length > 0) {
        selectDeploy(deployList[0]);
      } else {
        startNewDeploy();
      }
    } catch (err: any) {
      setError(err.message || '加载部署配置失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (open && app) {
      loadData();
    }
  }, [open, app]);

  const selectDeploy = (dep: AppDeploy) => {
    setActiveDeploy(dep);
    setIsNew(false);
    setName(dep.name || '');
    setBranchName(dep.branch_name || '');
    setBuildTag(dep.build_tag || '');
    setAutoPub(dep.auto_pub === 1);
    setApiUri(dep.api_uri || '');
    setApiKeyId(dep.api_key_id || 0);
    setEnvs(dep.envs || '');
    setError('');
    setSuccess('');
  };

  const startNewDeploy = () => {
    setActiveDeploy(null);
    setIsNew(true);
    setName('');
    setBranchName(branches.length > 0 ? branches[0].name : 'master');
    setBuildTag('');
    setAutoPub(false);
    setApiUri('');
    setApiKeyId(0);
    setEnvs('');
    setError('');
    setSuccess('');
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!app) return;
    if (!name.trim()) {
      setError('请输入部署环境名称');
      return;
    }

    setSaving(true);
    setError('');
    setSuccess('');
    try {
      const payload = {
        name: name.trim(),
        branch_name: branchName.trim(),
        build_tag: buildTag.trim(),
        auto_pub: autoPub ? 1 : 0,
        api_uri: apiUri.trim(),
        api_key_id: apiKeyId,
        envs: envs.trim(),
      };

      if (isNew) {
        const created = await appService.createDeploy(app.number, payload);
        setSuccess('新建部署环境成功');
        await loadData();
        selectDeploy(created);
      } else if (activeDeploy) {
        await appService.saveDeploy(app.number, { ...payload, id: activeDeploy.id });
        setSuccess('保存成功');
        await loadData();
      }
    } catch (err: any) {
      setError(err.message || '保存部署配置失败');
    } finally {
      setSaving(false);
    }
  };

  const handleCopy = async () => {
    if (!app || !activeDeploy) return;
    setSaving(true);
    try {
      const copied = await appService.copyDeploy(app.number, activeDeploy.id);
      setSuccess('复制环境成功');
      await loadData();
      selectDeploy(copied);
    } catch (err: any) {
      setError(err.message || '复制失败');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!app || !activeDeploy) return;
    const ok = await modalConfirm({
      title: '确认删除部署环境',
      description: `确定要删除部署环境 "${activeDeploy.name}" 吗？此操作无法撤销。`,
      variant: 'destructive',
      confirmText: '确认删除',
      cancelText: '取消',
    });
    if (!ok) return;

    setSaving(true);
    try {
      await appService.deleteDeploy(app.number, activeDeploy.id);
      setSuccess('删除成功');
      await loadData();
    } catch (err: any) {
      setError(err.message || '删除失败');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[780px] max-h-[85vh] flex flex-col p-0 overflow-hidden">
        <DialogHeader className="px-6 pt-6 pb-4 border-b border-border">
          <DialogTitle className="text-xl font-semibold flex items-center gap-2">
            <Server className="w-5 h-5 text-primary" />
            编译环境管理
            {app && <span className="text-sm font-normal text-muted-foreground">({app.name} - {app.number})</span>}
          </DialogTitle>
        </DialogHeader>

        <div className="flex-1 flex overflow-hidden min-h-[420px]">
          {/* 左侧环境列表 */}
          <div className="w-56 border-r border-border bg-muted/20 p-3 flex flex-col justify-between">
            <div className="space-y-1">
              <div className="text-xs font-semibold uppercase tracking-wider text-muted-foreground px-2 py-1">
                环境列表 ({deploys.length})
              </div>
              <div className="space-y-1 overflow-y-auto max-h-[340px]">
                {deploys.map((dep) => {
                  const isSelected = !isNew && activeDeploy?.id === dep.id;
                  return (
                    <button
                      key={dep.id}
                      type="button"
                      onClick={() => selectDeploy(dep)}
                      className={`w-full text-left px-3 py-2 rounded-md text-sm transition-colors flex items-center justify-between ${isSelected
                        ? 'bg-primary text-primary-foreground font-medium shadow-xs'
                        : 'hover:bg-muted text-foreground'
                        }`}
                    >
                      <span className="truncate">{dep.name}</span>
                      {dep.auto_pub === 1 && (
                        <span className={`text-[10px] px-1 py-0.5 rounded ${isSelected ? 'bg-primary-foreground/20 text-white' : 'bg-green-100 text-green-700 dark:bg-green-950 dark:text-green-300'}`}>
                          自动
                        </span>
                      )}
                    </button>
                  );
                })}
              </div>
            </div>

            <Button
              variant="outline"
              size="sm"
              className="w-full mt-2 gap-1"
              onClick={startNewDeploy}
            >
              <Plus className="w-4 h-4" />
              新建环境
            </Button>
          </div>

          {/* 右侧表单 */}
          <div className="flex-1 p-6 overflow-y-auto">
            {error && (
              <div className="mb-4 p-3 text-sm text-red-600 bg-red-50 dark:bg-red-950/40 rounded-md border border-red-200">
                {error}
              </div>
            )}
            {success && (
              <div className="mb-4 p-3 text-sm text-green-600 bg-green-50 dark:bg-green-950/40 rounded-md border border-green-200 flex items-center gap-1.5">
                <CheckCircle2 className="w-4 h-4" />
                {success}
              </div>
            )}

            <form onSubmit={handleSave} className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label htmlFor="dep-name" className="text-sm font-medium">
                    环境名称 <span className="text-red-500">*</span>
                  </Label>
                  <Input
                    id="dep-name"
                    placeholder="例如：开发测试环境 (Dev)"
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                  />
                </div>

                <div className="space-y-1.5">
                  <Label htmlFor="branch" className="text-sm font-medium flex items-center gap-1">
                    <GitBranch className="w-3.5 h-3.5 text-muted-foreground" />
                    关联代码分支
                  </Label>
                  {branches.length > 0 ? (
                    <Select value={branchName} onValueChange={setBranchName}>
                      <SelectTrigger id="branch">
                        <SelectValue placeholder="选择分支" />
                      </SelectTrigger>
                      <SelectContent>
                        {branches.map((b) => (
                          <SelectItem key={b.name} value={b.name}>
                            {b.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  ) : (
                    <Input
                      id="branch"
                      placeholder="master / main / release"
                      value={branchName}
                      onChange={(e) => setBranchName(e.target.value)}
                    />
                  )}
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label htmlFor="buildTag" className="text-sm font-medium">打包构建命令后缀 (Build Tag)</Label>
                  <Input
                    id="buildTag"
                    placeholder="例如：build:test 或 prod"
                    value={buildTag}
                    onChange={(e) => setBuildTag(e.target.value)}
                  />
                </div>

                <div className="space-y-1.5">
                  <Label htmlFor="apiKeyId" className="text-sm font-medium">K8S API 密钥 ID</Label>
                  <Input
                    id="apiKeyId"
                    type="number"
                    placeholder="0"
                    value={apiKeyId.toString()}
                    onChange={(e) => setApiKeyId(parseInt(e.target.value, 10) || 0)}
                  />
                </div>
              </div>

              <div className="space-y-1.5">
                <Label htmlFor="apiUri" className="text-sm font-medium">K8S API 地址 / 凭证</Label>
                <Input
                  id="apiUri"
                  placeholder="https://k8s-api.cluster.local:6443"
                  value={apiUri}
                  onChange={(e) => setApiUri(e.target.value)}
                />
              </div>

              <div className="space-y-1.5">
                <Label htmlFor="envs" className="text-sm font-medium">环境变量 / 配置参数 (JSON 或 Key=Value)</Label>
                <Input
                  id="envs"
                  placeholder="ENV=production;NODE_ENV=prod"
                  value={envs}
                  onChange={(e) => setEnvs(e.target.value)}
                />
              </div>

              <div className="flex items-center justify-between p-3 rounded-lg border border-border bg-muted/30">
                <div className="space-y-0.5">
                  <div className="text-sm font-medium">自动发布 </div>
                  <div className="text-xs text-muted-foreground">自动将固件发布到物联网平台</div>
                </div>
                <Switch checked={autoPub} onCheckedChange={setAutoPub} />
              </div>

              <div className="pt-4 flex items-center justify-between border-t border-border">
                <div>
                  {!isNew && activeDeploy && (
                    <div className="flex gap-2">
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={handleCopy}
                        disabled={saving}
                        className="gap-1"
                      >
                        <Copy className="w-3.5 h-3.5" />
                        复制环境
                      </Button>
                      <Button
                        type="button"
                        variant="destructive"
                        size="sm"
                        onClick={handleDelete}
                        disabled={saving}
                        className="gap-1"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                        删除环境
                      </Button>
                    </div>
                  )}
                </div>

                <div className="flex gap-2">
                  <Button
                    type="button"
                    variant="outline"
                    onClick={() => onOpenChange(false)}
                  >
                    关闭
                  </Button>
                  <Button type="submit" disabled={saving}>
                    {saving ? '保存中...' : isNew ? '创建环境' : '保存修改'}
                  </Button>
                </div>
              </div>
            </form>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
