import { useState, useEffect, useMemo } from 'react';
import { AppItem, AppUser } from '@/types/app';
import { useAuth } from '@/context/AuthContext';
import { useModalDialog } from '@/context/ModalDialogContext';
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
import { Checkbox } from '@/components/ui/checkbox';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import {
  Users,
  UserPlus,
  Trash2,
  Search,
  Check,
  ChevronsUpDown,
  X,
  Loader2,
  PenTool,
  ListFilter,
  CheckSquare,
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

interface AppMembersModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  app: AppItem | null;
  appService: any;
}

interface CandidateUser {
  id: string;
  name: string;
  email: string;
  role?: string;
  avatar_url?: string;
}

export function AppMembersModal({
  open,
  onOpenChange,
  app,
  appService,
}: AppMembersModalProps) {
  const { tenant } = useAuth();
  const { confirm: modalConfirm } = useModalDialog();
  const [members, setMembers] = useState<AppUser[]>([]);
  const [candidates, setCandidates] = useState<CandidateUser[]>([]);
  const [loading, setLoading] = useState(false);
  const [loadingCandidates, setLoadingCandidates] = useState(false);
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState('');

  // 模式与多选选中状态
  const [isManual, setIsManual] = useState(false);
  const [selectedUsers, setSelectedUsers] = useState<CandidateUser[]>([]);
  const [popoverOpen, setPopoverOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  // 手动输入模式下的备用状态
  const [manualUname, setManualUname] = useState('');
  const [manualUid, setManualUid] = useState('');

  // 权限角色
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

  const loadCandidates = async () => {
    setLoadingCandidates(true);
    try {
      const list = await appService.getUserCandidates(tenant?.id);
      setCandidates(list || []);
    } catch (err) {
      console.error('Failed to load candidate users', err);
    } finally {
      setLoadingCandidates(false);
    }
  };

  useEffect(() => {
    if (open && app) {
      loadMembers();
      loadCandidates();
      setSelectedUsers([]);
      setSearchQuery('');
      setManualUname('');
      setManualUid('');
      setError('');
    }
  }, [open, app]);

  // 过滤候选用户
  const filteredCandidates = useMemo(() => {
    if (!searchQuery.trim()) return candidates;
    const q = searchQuery.toLowerCase();
    return candidates.filter(
      (u) =>
        u.name?.toLowerCase().includes(q) ||
        u.email?.toLowerCase().includes(q) ||
        u.id?.toLowerCase().includes(q)
    );
  }, [candidates, searchQuery]);

  // 判断用户是否已是成员
  const isMember = (userId: string, userName: string) => {
    return members.some(
      (m) => String(m.uid) === String(userId) || m.uname === userName
    );
  };

  // 多选切换
  const toggleUser = (u: CandidateUser) => {
    setSelectedUsers((prev) => {
      const exists = prev.some((item) => item.id === u.id);
      if (exists) {
        return prev.filter((item) => item.id !== u.id);
      } else {
        return [...prev, u];
      }
    });
  };

  // 移除单个已选项
  const removeSelectedUser = (userId: string) => {
    setSelectedUsers((prev) => prev.filter((item) => item.id !== userId));
  };

  // 全选当前过滤结果中未加入项目的用户
  const handleSelectAllFiltered = () => {
    const available = filteredCandidates.filter(
      (u) => !isMember(u.id, u.name)
    );
    setSelectedUsers((prev) => {
      const prevIds = new Set(prev.map((item) => item.id));
      const newlyAdded = available.filter((u) => !prevIds.has(u.id));
      return [...prev, ...newlyAdded];
    });
  };

  // 清空已选
  const handleClearSelected = () => {
    setSelectedUsers([]);
  };

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!app) return;

    if (isManual) {
      if (!manualUname.trim()) {
        setError('请输入用户名 / 成员姓名');
        return;
      }
      const targetUid = manualUid.trim() || `${Date.now() % 100000}`;
      const targetUname = manualUname.trim();

      if (isMember(targetUid, targetUname)) {
        setError(`成员 "${targetUname}" 已在项目中，请勿重复添加`);
        return;
      }

      setAdding(true);
      setError('');
      try {
        await appService.createAppUser(app.number, {
          uid: targetUid,
          uname: targetUname,
          key: roleKey,
        });
        setManualUname('');
        setManualUid('');
        await loadMembers();
      } catch (err: any) {
        setError(err.message || '添加成员失败');
      } finally {
        setAdding(false);
      }
    } else {
      if (selectedUsers.length === 0) {
        setError('请在用户列表中选择要添加的成员（可多选）');
        return;
      }

      setAdding(true);
      setError('');
      try {
        const results = await Promise.allSettled(
          selectedUsers.map((u) =>
            appService.createAppUser(app.number, {
              uid: u.id,
              uname: u.name,
              key: roleKey,
            })
          )
        );

        const failed = results.filter((r) => r.status === 'rejected');
        if (failed.length > 0) {
          setError(`部分成员添加失败（${failed.length}/${selectedUsers.length}）`);
        }

        // 清空已添加成功的用户
        const successIndexes = results
          .map((r, idx) => (r.status === 'fulfilled' ? idx : -1))
          .filter((idx) => idx !== -1);
        const successIds = new Set(successIndexes.map((idx) => selectedUsers[idx].id));
        setSelectedUsers((prev) => prev.filter((u) => !successIds.has(u.id)));

        await loadMembers();
      } catch (err: any) {
        setError(err.message || '添加成员失败');
      } finally {
        setAdding(false);
      }
    }
  };

  const handleRemove = async (m: AppUser) => {
    if (!app) return;
    const ok = await modalConfirm({
      title: '确认移出成员',
      description: `确定要将成员 "${m.uname}" (${m.uid}) 移出当前项目吗？`,
      variant: 'destructive',
      confirmText: '确认移出',
      cancelText: '取消',
    });
    if (!ok) return;

    try {
      await appService.deleteAppUser(app.number, m.id);
      await loadMembers();
    } catch (err: any) {
      setError(err.message || '移除成员失败');
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[640px]">
        <DialogHeader>
          <DialogTitle className="text-xl font-semibold flex items-center gap-2">
            <Users className="w-5 h-5 text-primary" />
            参与成员管理
            {app && (
              <span className="text-sm font-normal text-muted-foreground">
                ({app.name})
              </span>
            )}
          </DialogTitle>
        </DialogHeader>

        {error && (
          <div className="p-3 text-sm text-red-600 bg-red-50 dark:bg-red-950/40 rounded-md border border-red-200 dark:border-red-900">
            {error}
          </div>
        )}

        {/* 添加成员表单 */}
        <form
          onSubmit={handleAdd}
          className="p-3.5 rounded-lg border border-border bg-muted/20 space-y-3"
        >
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
              {isManual ? '手动录入新成员' : '从现有系统用户添加'}
            </span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => {
                setIsManual(!isManual);
                setError('');
              }}
              className="h-6 text-xs text-primary hover:text-primary hover:bg-primary/10 gap-1 px-2"
            >
              {isManual ? (
                <>
                  <ListFilter className="w-3 h-3" />
                  从用户列表中选择
                </>
              ) : (
                <>
                  <PenTool className="w-3 h-3" />
                  手动录入 UID
                </>
              )}
            </Button>
          </div>

          <div className="flex gap-3 items-end">
            {!isManual ? (
              /* 从用户列表选择下拉框（支持多选） */
              <div className="flex-1 space-y-1">
                <Label className="text-xs font-medium flex items-center justify-between">
                  <span>选择成员 (支持多选)</span>
                  {selectedUsers.length > 0 && (
                    <span className="text-[11px] text-primary font-normal">
                      已选 {selectedUsers.length} 人
                    </span>
                  )}
                </Label>
                <Popover open={popoverOpen} onOpenChange={setPopoverOpen}>
                  <PopoverTrigger asChild>
                    <Button
                      variant="outline"
                      role="combobox"
                      aria-expanded={popoverOpen}
                      className="w-full min-h-9 h-auto py-1 justify-between font-normal bg-background text-sm px-2.5"
                    >
                      {selectedUsers.length > 0 ? (
                        <div className="flex items-center gap-1.5 flex-wrap min-w-0 max-w-[calc(100%-48px)]">
                          {selectedUsers.slice(0, 2).map((u) => (
                            <Badge
                              key={u.id}
                              variant="secondary"
                              className="h-6 px-1.5 gap-1 text-xs font-normal bg-muted/90"
                            >
                              <div className="w-3.5 h-3.5 rounded-full bg-primary/20 text-primary text-[9px] font-semibold flex items-center justify-center shrink-0">
                                {u.name?.charAt(0)?.toUpperCase() || 'U'}
                              </div>
                              <span className="max-w-[75px] truncate">{u.name}</span>
                              <span
                                onClick={(e) => {
                                  e.stopPropagation();
                                  removeSelectedUser(u.id);
                                }}
                                className="hover:bg-background/80 rounded-full p-0.5 cursor-pointer text-muted-foreground hover:text-foreground"
                              >
                                <X className="w-3 h-3" />
                              </span>
                            </Badge>
                          ))}
                          {selectedUsers.length > 2 && (
                            <Badge
                              variant="outline"
                              className="h-6 px-1.5 text-xs text-muted-foreground"
                            >
                              +{selectedUsers.length - 2} 人
                            </Badge>
                          )}
                        </div>
                      ) : (
                        <span className="text-muted-foreground text-xs">
                          {loadingCandidates
                            ? '正在加载用户列表...'
                            : '点击从系统用户列表中选择成员 (可多选)...'}
                        </span>
                      )}
                      <div className="flex items-center gap-1 shrink-0 ml-1">
                        {selectedUsers.length > 0 && (
                          <span
                            onClick={(e) => {
                              e.stopPropagation();
                              handleClearSelected();
                            }}
                            title="清空已选"
                            className="p-0.5 hover:bg-muted rounded text-muted-foreground hover:text-foreground cursor-pointer"
                          >
                            <X className="w-3.5 h-3.5" />
                          </span>
                        )}
                        <ChevronsUpDown className="w-3.5 h-3.5 opacity-50" />
                      </div>
                    </Button>
                  </PopoverTrigger>
                  <PopoverContent
                    className="w-[380px] p-2 space-y-2"
                    align="start"
                  >
                    <div className="relative">
                      <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground" />
                      <Input
                        placeholder="搜索姓名、邮箱或编号..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        className="h-8 pl-8 text-xs bg-muted/40"
                        autoFocus
                      />
                    </div>

                    <div className="flex items-center justify-between px-1 text-xs text-muted-foreground">
                      <span>
                        已选择{' '}
                        <strong className="text-foreground">
                          {selectedUsers.length}
                        </strong>{' '}
                        人
                      </span>
                      <div className="flex items-center gap-1">
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          onClick={handleSelectAllFiltered}
                          className="h-6 px-1.5 text-xs text-primary hover:text-primary gap-1"
                        >
                          <CheckSquare className="w-3 h-3" />
                          全选当前
                        </Button>
                        {selectedUsers.length > 0 && (
                          <Button
                            type="button"
                            variant="ghost"
                            size="sm"
                            onClick={handleClearSelected}
                            className="h-6 px-1.5 text-xs text-muted-foreground hover:text-destructive"
                          >
                            清空
                          </Button>
                        )}
                      </div>
                    </div>

                    <div className="max-h-56 overflow-y-auto divide-y divide-border/40">
                      {loadingCandidates ? (
                        <div className="py-6 text-center text-xs text-muted-foreground flex items-center justify-center gap-2">
                          <Loader2 className="w-4 h-4 animate-spin text-primary" />
                          加载用户列表中...
                        </div>
                      ) : filteredCandidates.length === 0 ? (
                        <div className="py-6 text-center text-xs text-muted-foreground">
                          未找到匹配的用户
                        </div>
                      ) : (
                        filteredCandidates.map((u) => {
                          const alreadyMember = isMember(u.id, u.name);
                          const isSelected = selectedUsers.some((item) => item.id === u.id);
                          return (
                            <div
                              key={u.id}
                              onClick={() => {
                                if (alreadyMember) return;
                                toggleUser(u);
                              }}
                              className={`flex items-center justify-between p-2 rounded text-xs transition-colors select-none ${
                                alreadyMember
                                  ? 'opacity-40 cursor-not-allowed bg-muted/20'
                                  : isSelected
                                    ? 'bg-primary/10 text-primary cursor-pointer'
                                    : 'hover:bg-accent cursor-pointer'
                              }`}
                            >
                              <div className="flex items-center gap-2.5 min-w-0">
                                <Checkbox
                                  checked={isSelected}
                                  disabled={alreadyMember}
                                  onCheckedChange={() => {
                                    if (alreadyMember) return;
                                    toggleUser(u);
                                  }}
                                  onClick={(e) => e.stopPropagation()}
                                />
                                <div className="w-6 h-6 rounded-full bg-primary/10 text-primary font-semibold flex items-center justify-center shrink-0">
                                  {u.name?.charAt(0)?.toUpperCase() || 'U'}
                                </div>
                                <div className="min-w-0 text-left">
                                  <div className="font-medium text-foreground truncate">
                                    {u.name}
                                  </div>
                                  <div className="text-[11px] text-muted-foreground truncate">
                                    {u.email}
                                  </div>
                                </div>
                              </div>
                              <div className="shrink-0 ml-2">
                                {alreadyMember ? (
                                  <Badge
                                    variant="outline"
                                    className="text-[10px] text-muted-foreground border-border"
                                  >
                                    已在项目中
                                  </Badge>
                                ) : isSelected ? (
                                  <Check className="w-4 h-4 text-primary" />
                                ) : null}
                              </div>
                            </div>
                          );
                        })
                      )}
                    </div>
                  </PopoverContent>
                </Popover>
              </div>
            ) : (
              /* 手动输入模式 */
              <>
                <div className="flex-1 space-y-1">
                  <Label htmlFor="uname" className="text-xs font-medium">
                    用户名 / 成员姓名
                  </Label>
                  <Input
                    id="uname"
                    placeholder="例如：张三"
                    value={manualUname}
                    onChange={(e) => setManualUname(e.target.value)}
                    className="h-9 text-sm"
                  />
                </div>

                <div className="w-36 space-y-1">
                  <Label htmlFor="uid" className="text-xs font-medium">
                    用户编号 / UID
                  </Label>
                  <Input
                    id="uid"
                    type="text"
                    placeholder="1001 或 UUID"
                    value={manualUid}
                    onChange={(e) => setManualUid(e.target.value)}
                    className="h-9 text-xs font-mono"
                  />
                </div>
              </>
            )}

            <div className="w-32 space-y-1">
              <Label htmlFor="roleKey" className="text-xs font-medium">
                参与者角色
              </Label>
              <Select value={roleKey} onValueChange={setRoleKey}>
                <SelectTrigger id="roleKey" className="h-9 text-sm bg-background">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="leader">负责人</SelectItem>
                  <SelectItem value="developer">开发者</SelectItem>
                  <SelectItem value="tester">测试人员</SelectItem>
                  <SelectItem value="pm">产品经理</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <Button
              type="submit"
              size="sm"
              className="h-9 px-4 gap-1.5 shrink-0"
              disabled={adding || (!isManual && selectedUsers.length === 0)}
            >
              {adding ? (
                <Loader2 className="w-3.5 h-3.5 animate-spin" />
              ) : (
                <UserPlus className="w-3.5 h-3.5" />
              )}
              {isManual
                ? '添加'
                : selectedUsers.length > 1
                  ? `批量添加 (${selectedUsers.length})`
                  : '添加'}
            </Button>
          </div>
        </form>

        {/* 成员列表 */}
        <div className="rounded-md border border-border overflow-hidden mt-1">
          <Table>
            <TableHeader className="bg-muted/40">
              <TableRow>
                <TableHead className="w-24">UID</TableHead>
                <TableHead>成员姓名</TableHead>
                <TableHead>参与者角色</TableHead>
                <TableHead className="w-16 text-right">操作</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {loading ? (
                <TableRow>
                  <TableCell
                    colSpan={4}
                    className="text-center py-6 text-muted-foreground"
                  >
                    <div className="flex items-center justify-center gap-2">
                      <Loader2 className="w-4 h-4 animate-spin text-primary" />
                      加载成员中...
                    </div>
                  </TableCell>
                </TableRow>
              ) : members.length === 0 ? (
                <TableRow>
                  <TableCell
                    colSpan={4}
                    className="text-center py-6 text-muted-foreground"
                  >
                    暂无成员绑定，请在上方选择并添加成员
                  </TableCell>
                </TableRow>
              ) : (
                members.map((m) => {
                  const getRoleInfo = (key?: string) => {
                    switch (key) {
                      case 'leader':
                      case 'admin':
                      case 'owner':
                        return { label: '负责人', variant: 'default' as const };
                      case 'developer':
                        return { label: '开发者', variant: 'secondary' as const };
                      case 'tester':
                      case 'qa':
                        return { label: '测试人员', variant: 'outline' as const };
                      case 'pm':
                      case 'product_manager':
                        return { label: '产品经理', variant: 'outline' as const };
                      case 'reporter':
                        return { label: '只读访问', variant: 'outline' as const };
                      default:
                        return { label: key || '开发者', variant: 'outline' as const };
                    }
                  };
                  const roleInfo = getRoleInfo(m.key);

                  return (
                    <TableRow key={m.id}>
                      <TableCell className="font-mono text-xs text-muted-foreground truncate max-w-[120px]" title={String(m.uid)}>
                        {String(m.uid).length > 12 ? `${String(m.uid).slice(0, 8)}...` : m.uid}
                      </TableCell>
                      <TableCell className="font-medium">
                        <div className="flex items-center gap-2">
                          <div className="w-6 h-6 rounded-full bg-primary/10 text-primary font-semibold flex items-center justify-center text-xs">
                            {m.uname?.charAt(0)?.toUpperCase() || 'U'}
                          </div>
                          <span>{m.uname}</span>
                        </div>
                      </TableCell>
                      <TableCell>
                        <Badge
                          variant={roleInfo.variant}
                          className="capitalize text-xs font-normal"
                        >
                          {roleInfo.label}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right">
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => handleRemove(m)}
                          className="text-destructive hover:text-destructive hover:bg-destructive/10 h-7 w-7 p-0"
                          title="移出项目"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </Button>
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

