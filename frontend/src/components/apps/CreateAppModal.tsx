import { useState } from 'react';
import { AppClass } from '@/types/app';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';

interface CreateAppModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  classList: AppClass[];
  onSubmit: (data: {
    name: string;
    key_name: string;
    desc: string;
    class_no: string;
    class_name: string;
    doc_path: string;
    gitlab_id: string;
    git_url: string;
    status: number;
  }) => Promise<void>;
}

export function CreateAppModal({
  open,
  onOpenChange,
  classList,
  onSubmit,
}: CreateAppModalProps) {
  const [name, setName] = useState('');
  const [keyName, setKeyName] = useState('');
  const [desc, setDesc] = useState('');
  const [classNo, setClassNo] = useState('');
  const [docPath, setDocPath] = useState('');
  const [gitlabId, setGitlabId] = useState('');
  const [gitUrl, setGitUrl] = useState('');
  const [status, setStatus] = useState<number>(2);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      setError('请输入应用名称');
      return;
    }
    if (!keyName.trim()) {
      setError('请输入唯一英文标识 (KeyName)');
      return;
    }

    const selectedClass = classList.find((c) => c.number === classNo);
    const className = selectedClass ? selectedClass.name : '';

    setSubmitting(true);
    setError('');
    try {
      await onSubmit({
        name: name.trim(),
        key_name: keyName.trim(),
        desc: desc.trim(),
        class_no: classNo,
        class_name: className,
        doc_path: docPath.trim(),
        gitlab_id: gitlabId.trim(),
        git_url: gitUrl.trim(),
        status,
      });
      // reset
      setName('');
      setKeyName('');
      setDesc('');
      setClassNo('');
      setDocPath('');
      setGitlabId('');
      setGitUrl('');
      onOpenChange(false);
    } catch (err: any) {
      setError(err.message || '创建失败');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[540px]">
        <DialogHeader>
          <DialogTitle className="text-xl font-semibold">新建应用</DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4 py-2">
          {error && (
            <div className="p-3 text-sm text-red-600 bg-red-50 dark:bg-red-950/40 dark:text-red-400 rounded-md border border-red-200 dark:border-red-900">
              {error}
            </div>
          )}

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <Label htmlFor="name" className="text-sm font-medium">
                应用名称 <span className="text-red-500">*</span>
              </Label>
              <Input
                id="name"
                placeholder="例如：OA协同系统"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="keyName" className="text-sm font-medium">
                唯一标识 (KeyName) <span className="text-red-500">*</span>
              </Label>
              <Input
                id="keyName"
                placeholder="例如：oa-system"
                value={keyName}
                onChange={(e) => setKeyName(e.target.value.toLowerCase().replace(/[^a-z0-9_-]/g, ''))}
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <Label htmlFor="classNo" className="text-sm font-medium">所属分类</Label>
              <Select value={classNo} onValueChange={setClassNo}>
                <SelectTrigger id="classNo">
                  <SelectValue placeholder="请选择分类" />
                </SelectTrigger>
                <SelectContent>
                  {classList.map((c) => (
                    <SelectItem key={c.number} value={c.number}>
                      {c.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="status" className="text-sm font-medium">应用状态</Label>
              <Select
                value={status.toString()}
                onValueChange={(val) => setStatus(parseInt(val, 10))}
              >
                <SelectTrigger id="status">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="2">正常运行</SelectItem>
                  <SelectItem value="1">已下线</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <Label htmlFor="gitlabId" className="text-sm font-medium">GitLab 项目 ID</Label>
              <Input
                id="gitlabId"
                placeholder="例如：128 或 group/project"
                value={gitlabId}
                onChange={(e) => setGitlabId(e.target.value)}
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="gitUrl" className="text-sm font-medium">Git 仓库地址</Label>
              <Input
                id="gitUrl"
                placeholder="https://gitlab.com/..."
                value={gitUrl}
                onChange={(e) => setGitUrl(e.target.value)}
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="docPath" className="text-sm font-medium">接口文档 / Swagger 地址</Label>
            <Input
              id="docPath"
              placeholder="https://api.example.com/swagger"
              value={docPath}
              onChange={(e) => setDocPath(e.target.value)}
            />
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="desc" className="text-sm font-medium">应用描述</Label>
            <Textarea
              id="desc"
              rows={3}
              placeholder="请输入简要业务描述..."
              value={desc}
              onChange={(e) => setDesc(e.target.value)}
            />
          </div>

          <DialogFooter className="pt-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={submitting}
            >
              取消
            </Button>
            <Button type="submit" disabled={submitting}>
              {submitting ? '创建中...' : '确认创建'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
