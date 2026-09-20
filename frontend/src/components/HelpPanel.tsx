import { X, Shield, Users, Briefcase, User } from 'lucide-react';
import { useTranslations } from '../context/I18nContext';

interface HelpPanelProps {
    isOpen: boolean;
    onClose: () => void;
}

export function HelpPanel({ isOpen, onClose }: HelpPanelProps) {
    const t = useTranslations('Companies');
    const tUsers = useTranslations('Users');
    if (!isOpen) return null;

    const isZh = t('rolesTitle') === '用户角色与权限说明';

    const roles = [
        {
            name: tUsers('roleSuperAdmin'),
            icon: Shield,
            color: 'text-purple-600 dark:text-purple-300',
            bgColor: 'bg-purple-50 dark:bg-purple-900/20',
            permissions: isZh ? [
                '管理系统内全部企业与多租户空间',
                '创建、编辑与暂停企业空间',
                '访问与维护全局系统数据',
                '配置系统底层与全局偏好',
                '拥有最高管理员控制权限'
            ] : [
                'Manage all companies/tenants',
                'Create and delete companies',
                'Access all company data',
                'Manage system settings',
                'Full administrative control'
            ]
        },
        {
            name: tUsers('roleAdmin'),
            icon: Users,
            color: 'text-blue-600 dark:text-blue-300',
            bgColor: 'bg-blue-50 dark:bg-blue-900/20',
            permissions: isZh ? [
                '管理所属企业成员及部门划分',
                '创建与管理我的分享上传任务',
                '查看企业内部 部门文件资源',
                '配置企业专属规则与合规模式',
                '无法跨企业访问其他租户空间'
            ] : [
                'Manage company users',
                'Create file requests',
                'View all company files',
                'Manage company settings',
                'Cannot manage other companies'
            ]
        },
        {
            name: tUsers('roleManager'),
            icon: Briefcase,
            color: 'text-green-600 dark:text-green-300',
            bgColor: 'bg-green-50 dark:bg-green-900/20',
            permissions: isZh ? [
                '发起团队我的分享链接',
                '管理并查看团队部门文件',
                '上传、下载及审批文件',
                '与部门内外成员共享协作',
                '无企业级用户管理权限'
            ] : [
                'Create file requests',
                'View team files',
                'Upload and download files',
                'Share files with team',
                'Cannot manage users'
            ]
        },
        {
            name: tUsers('roleEmployee'),
            icon: User,
            color: 'text-gray-600 dark:text-gray-300',
            bgColor: 'bg-gray-50 dark:bg-gray-700',
            permissions: isZh ? [
                '通过有效收集链接上传文件',
                '查看分配给个人的文件与目录',
                '下载与查看被共享的文件',
                '基础文件个人操作',
                '受限只读与个人操作权限'
            ] : [
                'Upload files via requests',
                'View assigned files',
                'Download shared files',
                'Basic file operations',
                'Limited access'
            ]
        }
    ];

    return (
        <>
            {/* Backdrop */}
            <div
                className="fixed inset-0 bg-black/50 z-40 transition-opacity"
                onClick={onClose}
            />

            {/* Panel */}
            <div className="fixed right-0 top-0 h-full w-full max-w-md bg-white dark:bg-gray-800 shadow-2xl z-50 overflow-y-auto">
                {/* Header */}
                <div className="sticky top-0 bg-white dark:bg-gray-800 border-b border-gray-200 dark:border-gray-700 px-6 py-4 flex items-center justify-between">
                    <h2 className="text-xl font-semibold text-gray-900 dark:text-white">{t('rolesTitle')}</h2>
                    <button
                        onClick={onClose}
                        className="p-2 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
                    >
                        <X className="w-5 h-5 text-gray-500 dark:text-gray-400" />
                    </button>
                </div>

                {/* Content */}
                <div className="p-6 space-y-4">
                    {roles.map((role) => (
                        <div
                            key={role.name}
                            className="bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg p-4 hover:shadow-md transition-shadow"
                        >
                            <div className="flex items-start gap-3 mb-3">
                                <div className={`p-2 rounded-lg ${role.bgColor}`}>
                                    <role.icon className={`w-5 h-5 ${role.color}`} />
                                </div>
                                <div>
                                    <h3 className={`font-semibold ${role.color}`}>{role.name}</h3>
                                </div>
                            </div>
                            <ul className="space-y-2">
                                {role.permissions.map((permission, idx) => (
                                    <li key={idx} className="flex items-start gap-2 text-sm text-gray-600 dark:text-gray-300">
                                        <span className="text-gray-400 dark:text-gray-500 mt-0.5">•</span>
                                        <span>{permission}</span>
                                    </li>
                                ))}
                            </ul>
                        </div>
                    ))}
                </div>

                {/* Footer */}
                <div className="sticky bottom-0 bg-gray-50 dark:bg-gray-900/50 border-t border-gray-200 dark:border-gray-700 px-6 py-4">
                    <p className="text-sm text-gray-600 dark:text-gray-400 text-center">
                        {isZh ? '如需调整您的角色与访问权限，请联系企业超级管理员' : 'Contact your administrator to change your role'}
                    </p>
                </div>
            </div>
        </>
    );
}
