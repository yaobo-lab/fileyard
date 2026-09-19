import { useState, useEffect } from 'react';
import { AppItem, AppUser } from '@/types/app';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import { Users, UserPlus, Trash2, ShieldCheck } from 'lucide-react';
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

interface AppMembersModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  app: AppItem | null;
  appService: any;
}

export function AppMembersModal({
  open,
  onOpenChange,
  app,
  appService,
}: AppMembersModalProps) {
  const [members, setMembers] = useState<AppUser[]>([]);
  const [loading, setLoading] = useState(false);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState('');

  const [uname, setUname] = useState('');
  const [uid, setUid] = useState<string>('');
  const [roleKey, setRoleKey] = useState<string>('developer');

  const loadMembers = async () => {
    if (!app) return;
    setLoading(true);
    setError('');
    try {
      const list = await appService.getAppUsers(app.number);
      setMembers(list);
    } catch (err: any) {
      setError(err.message || '加载成员失败');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (open && app) {
      loadMembers();
    }
  }, [open, app]);

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!app) return;
    if (!uname.trim()) {
      setError('请输入用户名');
      return;
    }

    setAdding(true);
    setError('');
    try {
      const userId = parseInt(uid, 10) || Date.now() % 100000;
      await appService.createAppUser(app.number, {
        uid: userId,
        uname: uname.trim(),
        key: roleKey,
      });
      setUname('');
      setUid('');
      await loadMembers();
    } catch (err: any) {
      setError(err.message || '添加成员失败');
    } finally {
      setAdding(false);
    }
  };

  const handleRemove = async (id: number) => {
    if (!app) return;
    try {
      await appService.deleteAppUser(app.number, id);
      await loadMembers();
    } catch (err: any) {
      setError(err.message || '移除成员失败');
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[620px]">
        <DialogHeader>
          <DialogTitle className="text-xl font-semibold flex items-center gap-2">
            <Users className="w-5 h-5 text-primary" />
            项目成员权限管理
            {app && <span className="text-sm font-normal text-muted-foreground">({app.name})</span>}
          </DialogTitle>
        </DialogHeader>

        {error && (
          <div className="p-3 text-sm text-red-600 bg-red-50 dark:bg-red-950/40 rounded-md border border-red-200">
            {error}
          </div>
        )}

        {/* 添加成员表单 */}
        <form onSubmit={handleAdd} className="flex gap-3 items-end p-3 rounded-lg border border-border bg-muted/20">
          <div className="flex-1 space-y-1">
            <Label htmlFor="uname" className="text-xs font-medium">用户名 / 成员姓名</Label>
            <Input
              id="uname"
              placeholder="例如：张三"
              value={uname}
              onChange={(e) => setUname(e.target.value)}
              className="h-8 text-sm"
            />
          </div>

          <div className="w-24 space-y-1">
            <Label htmlFor="uid" className="text-xs font-medium">用户编号</Label>
            <Input
              id="uid"
              type="number"
              placeholder="1001"
              value={uid}
              onChange={(e) => setUid(e.target.value)}
              className="h-8 text-sm"
            />
          </div>

          <div className="w-32 space-y-1">
            <Label htmlFor="roleKey" className="text-xs font-medium">权限角色</Label>
            <Select value={roleKey} onValueChange={setRoleKey}>
              <SelectTrigger id="roleKey" className="h-8 text-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="admin">管理员</SelectItem>
                <SelectItem value="developer">开发者</SelectItem>
                <SelectItem value="reporter">只读访问</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <Button type="submit" size="sm" className="h-8 gap-1" disabled={adding}>
            <UserPlus className="w-3.5 h-3.5" />
            添加
          </Button>
        </form>

        {/* 成员列表 */}
        <div className="rounded-md border border-border overflow-hidden mt-2">
          <Table>
            <TableHeader className="bg-muted/40">
              <TableRow>
                <TableHead className="w-20">UID</TableHead>
                <TableHead>成员姓名</TableHead>
                <TableHead>权限</TableHead>
                <TableHead className="w-20 text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell colSpan={4} className="text-center py-6 text-muted-foreground">
                    加载成员中...
                  </TableCell>
                </TableRow>
              ) : members.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={4} className="text-center py-6 text-muted-foreground">
                    暂无成员绑定，请在上方添加
                  </TableCell>
                </TableRow>
              ) : (
                members.map((m) => (
                  <TableRow key={m.id}>
                    <TableCell className="font-mono text-xs">{m.uid}</TableCell>
                    <TableCell className="font-medium">{m.uname}</TableCell>
                    <TableCell>
                      <Badge variant={m.key === 'admin' ? 'default' : 'secondary'} className="capitalize">
                        {m.key}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleRemove(m.id)}
                        className="text-destructive hover:text-destructive hover:bg-destructive/10 h-7 w-7 p-0"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </div>
      </DialogContent>
    </Dialog>
  );
}
