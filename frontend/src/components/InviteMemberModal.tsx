import { useState } from 'react';
import { X, Mail, Link2, Copy, Check, Send, Sparkles, Building2, Clock, Users, Shield } from 'lucide-react';
import clsx from 'clsx';
import { useTranslations } from '../context/I18nContext';
import { copyToClipboard } from '../lib/utils';

interface InviteMemberModalProps {
    isOpen: boolean;
    onClose: () => void;
    companyId: string;
    companyName: string;
    departments: { id: string; name: string }[];
}

export function InviteMemberModal({
    isOpen,
    onClose,
    companyId,
    companyName,
    departments,
}: InviteMemberModalProps) {
    const t = useTranslations('CompanyDetails');
    const tUsers = useTranslations('Users');
    const tCommon = useTranslations('Common');

    const [activeTab, setActiveTab] = useState<'email' | 'link'>('email');

    // Email invite state
    const [emails, setEmails] = useState('');
    const [role, setRole] = useState('Employee');
    const [departmentId, setDepartmentId] = useState('');
    const [inviteNote, setInviteNote] = useState('');
    const [isSending, setIsSending] = useState(false);
    const [sendSuccess, setSendSuccess] = useState(false);

    // Link invite state
    const [expiryDays, setExpiryDays] = useState('7');
    const [isCopied, setIsCopied] = useState(false);

    if (!isOpen) return null;

    const inviteLink = `${window.location.origin}/register?invite=${companyId}&expires=${expiryDays}`;

    const handleCopyLink = async () => {
        const ok = await copyToClipboard(inviteLink);
        if (ok) {
            setIsCopied(true);
            setTimeout(() => setIsCopied(false), 2500);
        }
    };

    const handleSendInvite = (e: React.FormEvent) => {
        e.preventDefault();
        if (!emails.trim()) return;
        setIsSending(true);
        // Simulate sending invite email with nice feedback
        setTimeout(() => {
            setIsSending(false);
            setSendSuccess(true);
            setTimeout(() => {
                setSendSuccess(false);
                setEmails('');
                setInviteNote('');
                onClose();
            }, 2000);
        }, 600);
    };

    return (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/50 backdrop-blur-sm animate-in fade-in duration-150">
            <div className="bg-white dark:bg-gray-800 rounded-2xl shadow-2xl border border-gray-200 dark:border-gray-700 w-full max-w-lg overflow-hidden flex flex-col max-h-[90vh]">
                {/* Header */}
                <div className="px-6 py-4 border-b border-gray-200 dark:border-gray-700 flex items-center justify-between bg-gray-50/50 dark:bg-gray-900/30">
                    <div className="flex items-center gap-3">
                        <div className="p-2 rounded-xl bg-indigo-100 dark:bg-indigo-900/40 text-indigo-600 dark:text-indigo-400">
                            <Mail className="w-5 h-5" />
                        </div>
                        <div>
                            <h3 className="text-base font-semibold text-gray-900 dark:text-white flex items-center gap-2">
                                <span>{t('inviteMember') || '邀请成员加入'}</span>
                                <span className="text-xs font-medium px-2 py-0.5 rounded-md bg-indigo-50 dark:bg-indigo-900/20 text-indigo-700 dark:text-indigo-300 border border-indigo-200 dark:border-indigo-800">
                                    {companyName}
                                </span>
                            </h3>
                            <p className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                                {t('inviteDesc') || '通过邮件发送邀请函，或生成专属邀请链接分享给新成员'}
                            </p>
                        </div>
                    </div>
                    <button
                        onClick={onClose}
                        className="p-1.5 rounded-lg text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                    >
                        <X className="w-5 h-5" />
                    </button>
                </div>

                {/* Tabs */}
                <div className="px-6 pt-3 border-b border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 flex gap-6">
                    <button
                        type="button"
                        onClick={() => setActiveTab('email')}
                        className={clsx(
                            'pb-3 text-sm font-medium border-b-2 transition-colors flex items-center gap-2',
                            activeTab === 'email'
                                ? 'border-indigo-600 text-indigo-600 dark:text-indigo-400 font-semibold'
                                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400'
                        )}
                    >
                        <Mail className="w-4 h-4" />
                        <span>{t('inviteByEmail') || '邮箱邀请'}</span>
                    </button>
                    <button
                        type="button"
                        onClick={() => setActiveTab('link')}
                        className={clsx(
                            'pb-3 text-sm font-medium border-b-2 transition-colors flex items-center gap-2',
                            activeTab === 'link'
                                ? 'border-indigo-600 text-indigo-600 dark:text-indigo-400 font-semibold'
                                : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400'
                        )}
                    >
                        <Link2 className="w-4 h-4" />
                        <span>{t('inviteByLink') || '邀请链接'}</span>
                    </button>
                </div>

                {/* Tab 1: Email Invite */}
                {activeTab === 'email' && (
                    <form onSubmit={handleSendInvite} className="p-6 space-y-4 flex-1 overflow-y-auto">
                        <div>
                            <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                                {t('inviteEmailsLabel') || '接收邀请的邮箱地址'} <span className="text-red-500">*</span>
                            </label>
                            <textarea
                                required
                                rows={3}
                                value={emails}
                                onChange={(e) => setEmails(e.target.value)}
                                placeholder="输入被邀请人邮箱，支持多个邮箱以逗号或换行分隔（例如：alice@company.com, bob@company.com）"
                                className="w-full px-3.5 py-2.5 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 outline-none resize-none"
                            />
                        </div>

                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                            <div>
                                <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    {tUsers('role') || '预分配角色'}
                                </label>
                                <select
                                    value={role}
                                    onChange={(e) => setRole(e.target.value)}
                                    className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 outline-none"
                                >
                                    <option value="Employee">{tUsers('roleEmployee') || '普通员工 (Employee)'}</option>
                                    <option value="Manager">{tUsers('roleManager') || '部门主管 (Manager)'}</option>
                                    <option value="Admin">{tUsers('roleAdmin') || '管理员 (Admin)'}</option>
                                </select>
                            </div>

                            <div>
                                <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                                    {tUsers('department') || '预分配部门'}
                                </label>
                                <select
                                    value={departmentId}
                                    onChange={(e) => setDepartmentId(e.target.value)}
                                    className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 outline-none"
                                >
                                    <option value="">{t('unassignedDept') || '未分配部门'}</option>
                                    {departments.map(dept => (
                                        <option key={dept.id} value={dept.id}>{dept.name}</option>
                                    ))}
                                </select>
                            </div>
                        </div>

                        <div>
                            <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                                {t('inviteNote') || '邀请附言（可选）'}
                            </label>
                            <input
                                type="text"
                                value={inviteNote}
                                onChange={(e) => setInviteNote(e.target.value)}
                                placeholder="例如：欢迎加入 HDL / Demo2 团队！"
                                className="w-full px-3 py-2 text-sm border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-700 text-gray-900 dark:text-white focus:ring-2 focus:ring-indigo-500 outline-none"
                            />
                        </div>

                        {sendSuccess && (
                            <div className="p-3 rounded-lg bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-800 flex items-center gap-2 text-xs text-emerald-600 dark:text-emerald-400">
                                <Check className="w-4 h-4 shrink-0" />
                                <span>{t('inviteSuccess') || '邀请函已发送成功！被邀请人将在邮件中查收加入指引。'}</span>
                            </div>
                        )}

                        <div className="pt-2 flex justify-end gap-3">
                            <button
                                type="button"
                                onClick={onClose}
                                className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                            >
                                {tCommon('cancel') || '取消'}
                            </button>
                            <button
                                type="submit"
                                disabled={!emails.trim() || isSending || sendSuccess}
                                className={clsx(
                                    'flex items-center px-4 py-2 text-sm font-medium rounded-lg text-white transition-all shadow-sm',
                                    emails.trim() && !isSending && !sendSuccess
                                        ? 'bg-indigo-600 hover:bg-indigo-700 cursor-pointer'
                                        : 'bg-gray-300 dark:bg-gray-700 text-gray-400 cursor-not-allowed'
                                )}
                            >
                                {sendSuccess ? (
                                    <>
                                        <Check className="w-4 h-4 mr-1.5" />
                                        <span>{t('sent') || '已发送'}</span>
                                    </>
                                ) : (
                                    <>
                                        <Send className="w-4 h-4 mr-1.5" />
                                        <span>{isSending ? (t('sending') || '正在发送...') : (t('sendInvite') || '发送邀请')}</span>
                                    </>
                                )}
                            </button>
                        </div>
                    </form>
                )}

                {/* Tab 2: Link Invite */}
                {activeTab === 'link' && (
                    <div className="p-6 space-y-5 flex-1 overflow-y-auto">
                        <div>
                            <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                                {t('linkExpiry') || '邀请链接有效期'}
                            </label>
                            <div className="flex gap-3">
                                {[
                                    { value: '7', label: '7 天' },
                                    { value: '30', label: '30 天' },
                                    { value: '0', label: '永久有效' },
                                ].map(opt => (
                                    <button
                                        key={opt.value}
                                        type="button"
                                        onClick={() => setExpiryDays(opt.value)}
                                        className={clsx(
                                            'px-3.5 py-1.5 rounded-lg text-xs font-medium border transition-colors',
                                            expiryDays === opt.value
                                                ? 'border-indigo-600 bg-indigo-50 dark:bg-indigo-900/30 text-indigo-700 dark:text-indigo-300 font-semibold'
                                                : 'border-gray-200 dark:border-gray-700 text-gray-600 dark:text-gray-400 hover:border-gray-300'
                                        )}
                                    >
                                        {opt.label}
                                    </button>
                                ))}
                            </div>
                        </div>

                        <div>
                            <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1.5">
                                {t('inviteLink') || '专属企业邀请链接'}
                            </label>
                            <div className="flex gap-2">
                                <input
                                    type="text"
                                    readOnly
                                    value={inviteLink}
                                    className="flex-1 px-3.5 py-2 text-xs border border-gray-300 dark:border-gray-600 rounded-lg bg-gray-50 dark:bg-gray-900 text-gray-700 dark:text-gray-300 font-mono select-all outline-none"
                                />
                                <button
                                    type="button"
                                    onClick={handleCopyLink}
                                    className={clsx(
                                        'flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-xs font-medium text-white transition-all shrink-0',
                                        isCopied
                                            ? 'bg-emerald-600 hover:bg-emerald-700'
                                            : 'bg-indigo-600 hover:bg-indigo-700'
                                    )}
                                >
                                    {isCopied ? (
                                        <>
                                            <Check className="w-3.5 h-3.5" />
                                            <span>{t('copied') || '已复制'}</span>
                                        </>
                                    ) : (
                                        <>
                                            <Copy className="w-3.5 h-3.5" />
                                            <span>{t('copyLink') || '复制链接'}</span>
                                        </>
                                    )}
                                </button>
                            </div>
                        </div>

                        <div className="p-3.5 rounded-xl bg-indigo-50/50 dark:bg-indigo-950/20 border border-indigo-100 dark:border-indigo-900/30 space-y-1.5 text-xs text-indigo-900 dark:text-indigo-300">
                            <div className="flex items-center gap-2 font-medium">
                                <Sparkles className="w-4 h-4 text-indigo-500 shrink-0" />
                                <span>{t('inviteLinkTipTitle') || '使用提示'}</span>
                            </div>
                            <p className="text-gray-600 dark:text-gray-400 text-[11px] leading-relaxed">
                                {t('inviteLinkTipDesc') || '复制上方链接并通过邮件、企业微信、钉钉等方式直接发送给成员。成员打开链接完成注册后，将自动加入该企业空间并继承默认权限。'}
                            </p>
                        </div>

                        <div className="pt-2 flex justify-end">
                            <button
                                type="button"
                                onClick={onClose}
                                className="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                            >
                                {tCommon('close') || '关闭'}
                            </button>
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}
