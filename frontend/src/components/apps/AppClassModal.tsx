import { useState } from 'react';
import { useModalDialog } from '@/context/ModalDialogContext';
import { AppClass } from '@/types/app';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Layers, Plus, Pencil, Trash2, Check, X } from 'lucide-react';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';

interface AppClassModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  classList: AppClass[];
  onRefresh: () => Promise<void>;
  appService: any;
}

export function AppClassModal({
  open,
  onOpenChange,
  classList,
  onRefresh,
  appService,
}: AppClassModalProps) {
  const { confirm: modalConfirm } = useModalDialog();
  const [name, setName] = useState('');
  const [desc, setDesc] = useState('');
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editName, setEditName] = useState('');
  const [editDesc, setEditDesc] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;
    setLoading(true);
    setError('');
    try {
      await appService.saveClass({ name: name.trim(), desc: desc.trim() });
      setName('');
      setDesc('');
      await onRefresh();
    } catch (err: any) {
      setError(err.message || '添加分类失败');
    } finally {
      setLoading(false);
    }
  };

  const startEdit = (c: AppClass) => {
    setEditingId(c.id);
    setEditName(c.name);
    setEditDesc(c.desc || '');
  };

  const cancelEdit = () => {
    setEditingId(null);
  };

  const saveEdit = async (id: number) => {
    if (!editName.trim()) return;
    setLoading(true);
    try {
      await appService.saveClass({ id, name: editName.trim(), desc: editDesc.trim() });
      setEditingId(null);
      await onRefresh();
    } catch (err: any) {
      setError(err.message || '保存修改失败');
    } finally {
      setLoading(false);
    }
  };

  const handleDelete = async (id: number) => {
    const ok = await modalConfirm({
      title: '确认删除分类',
      description: '确定要删除该固件分类吗？此操作无法撤销。',
      variant: 'destructive',
      confirmText: '确认删除',
      cancelText: '取消',
    });
    if (!ok) return;

    try {
      await appService.deleteClass(id);
      await onRefresh();
    } catch (err: any) {
      setError(err.message || '删除失败');
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[640px]">
        <DialogHeader>
          <DialogTitle className="text-xl font-semibold flex items-center gap-2">
            <Layers className="w-5 h-5 text-primary" />
            固件分类管理
          </DialogTitle>
        </DialogHeader>

        {error && (
          <div className="p-3 text-sm text-red-600 bg-red-50 dark:bg-red-950/40 rounded-md border border-red-200">
            {error}
          </div>
        )}

        {/* 添加新分类 */}
        <form onSubmit={handleAdd} className="flex gap-2 items-end p-3 rounded-lg border border-border bg-muted/20">
          <div className="w-40 space-y-1">
            <Label htmlFor="cname" className="text-xs font-medium">分类名称</Label>
            <Input
              id="cname"
              placeholder="例如：基础服务"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="h-8 text-sm"
            />
          </div>

          <div className="flex-1 space-y-1">
            <Label htmlFor="cdesc" className="text-xs font-medium">描述说明</Label>
            <Input
              id="cdesc"
              placeholder="分类用途说明..."
              value={desc}
              onChange={(e) => setDesc(e.target.value)}
              className="h-8 text-sm"
            />
          </div>

          <Button type="submit" size="sm" className="h-8 gap-1" disabled={loading || !name.trim()}>
            <Plus className="w-3.5 h-3.5" />
            添加
          </Button>
        </form>

        {/* 分类列表 */}
        <div className="rounded-md border border-border overflow-hidden mt-2">
          <Table>
            <TableHeader className="bg-muted/40">
              <TableRow>
                <TableHead className="w-28">编号</TableHead>
                <TableHead className="w-36">名称</TableHead>
                <TableHead>描述</TableHead>
                <TableHead className="w-20 text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {classList.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={4} className="text-center py-6 text-muted-foreground">
                    暂无分类
                  </TableCell>
                </TableRow>
              ) : (
                classList.map((c) => {
                  const isEditing = editingId === c.id;
                  return (
                    <TableRow key={c.id}>
                      <TableCell className="font-mono text-xs text-muted-foreground">{c.number}</TableCell>
                      <TableCell>
                        {isEditing ? (
                          <Input
                            value={editName}
                            onChange={(e) => setEditName(e.target.value)}
                            className="h-7 text-xs"
                          />
                        ) : (
                          <span className="font-medium">{c.name}</span>
                        )}
                      </TableCell>
                      <TableCell>
                        {isEditing ? (
                          <Input
                            value={editDesc}
                            onChange={(e) => setEditDesc(e.target.value)}
                            className="h-7 text-xs"
                          />
                        ) : (
                          <span className="text-sm text-muted-foreground">{c.desc || '-'}</span>
                        )}
                      </TableCell>
                      <TableCell className="text-right">
                        {isEditing ? (
                          <div className="flex justify-end gap-1">
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => saveEdit(c.id)}
                              className="h-7 w-7 p-0 text-green-600 hover:text-green-700"
                            >
                              <Check className="w-3.5 h-3.5" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={cancelEdit}
                              className="h-7 w-7 p-0 text-muted-foreground"
                            >
                              <X className="w-3.5 h-3.5" />
                            </Button>
                          </div>
                        ) : (
                          <div className="flex justify-end gap-1">
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => startEdit(c)}
                              className="h-7 w-7 p-0 text-muted-foreground hover:text-foreground"
                            >
                              <Pencil className="w-3.5 h-3.5" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => handleDelete(c.id)}
                              className="h-7 w-7 p-0 text-destructive hover:text-destructive hover:bg-destructive/10"
                            >
                              <Trash2 className="w-3.5 h-3.5" />
                            </Button>
                          </div>
                        )}
                      </TableCell>
                    </TableRow>
                  );
                })
              )}
            </TableBody>
          </Table>
        </div>
      </DialogContent>
    </Dialog>
  );
}
