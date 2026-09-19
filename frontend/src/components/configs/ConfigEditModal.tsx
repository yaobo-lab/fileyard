import { useState, useEffect } from 'react';
import { AppConfig } from '@/types/app';
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

interface ConfigEditModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  config: AppConfig | null;
  onSubmit: (data: Partial<AppConfig>) => Promise<void>;
}

export function ConfigEditModal({
  open,
  onOpenChange,
  config,
  onSubmit,
}: ConfigEditModalProps) {
  const [name, setName] = useState('');
  const [key, setKey] = useState('');
  const [value, setValue] = useState('');
  const [remark, setRemark] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (config) {
      setName(config.name || '');
      setKey(config.key || '');
      setValue(config.value || '');
      setRemark(config.remark || '');
    } else {
      setName('');
      setKey('');
      setValue('');
      setRemark('');
    }
    setError('');
  }, [config, open]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) {
      setError('请输入配置名称');
      return;
    }
    if (!key.trim()) {
      setError('请输入配置 Key');
      return;
    }

    setSubmitting(true);
    setError('');
    try {
      await onSubmit({
        ...(config ? { id: config.id, number: config.number } : {}),
        name: name.trim(),
        key: key.trim(),
        value,
        remark: remark.trim(),
      });
      onOpenChange(false);
    } catch (err: any) {
      setError(err.message || '保存配置失败');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[620px] max-h-[85vh] flex flex-col p-0 overflow-hidden">
        <DialogHeader className="px-6 pt-6 pb-4 border-b border-border">
          <DialogTitle className="text-xl font-semibold">
            {config ? '编辑系统配置' : '新建系统配置'}
          </DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="flex-1 overflow-y-auto p-6 space-y-4">
          {error && (
            <div className="p-3 text-sm text-red-600 bg-red-50 dark:bg-red-950/40 rounded-md border border-red-200">
              {error}
            </div>
          )}

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <Label htmlFor="cfg-name" className="text-sm font-medium">
                配置名称 <span className="text-red-500">*</span>
              </Label>
              <Input
                id="cfg-name"
                placeholder="例如：CI 构建基础模板"
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="cfg-key" className="text-sm font-medium">
                配置 Key <span className="text-red-500">*</span>
              </Label>
              <Input
                id="cfg-key"
                placeholder="例如：ci-template-default"
                value={key}
                onChange={(e) => setKey(e.target.value)}
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="cfg-remark" className="text-sm font-medium">备注说明</Label>
            <Input
              id="cfg-remark"
              placeholder="配置用途说明..."
              value={remark}
              onChange={(e) => setRemark(e.target.value)}
            />
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="cfg-val" className="text-sm font-medium">
              配置内容 (YAML / JSON / 脚本)
            </Label>
            <Textarea
              id="cfg-val"
              rows={10}
              className="font-mono text-xs leading-relaxed bg-zinc-950 text-zinc-100 dark:bg-zinc-950"
              placeholder="version: '1.0'&#10;stages:&#10;  - build&#10;  - deploy"
              value={value}
              onChange={(e: React.ChangeEvent<HTMLTextAreaElement>) => setValue(e.target.value)}

            />
          </div>

          <DialogFooter className="pt-4 border-t border-border">
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={submitting}
            >
              取消
            </Button>
            <Button type="submit" disabled={submitting}>
              {submitting ? '保存中...' : '确认保存'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
