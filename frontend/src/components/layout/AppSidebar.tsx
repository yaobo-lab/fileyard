import * as React from 'react'
import { Link, useLocation } from 'react-router-dom'
import {
  LayoutDashboard,
  Users,
  FileText,
  Settings,
  Building2,
  Folder,
  Link2,
  Shield,
  Activity,
  Share2,
  CheckCircle,
  ExternalLink,
  Puzzle,
  Boxes,
  Sliders,
  Lock,
  Cpu,
  Radio,
  History,
  Send,
} from 'lucide-react'
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarTrigger,
  useSidebar,
} from '@/components/ui/sidebar'
import { TeamSwitcher } from './TeamSwitcher'
import { NavUser } from './NavUser'
import { useAuth } from '@/context/AuthContext'
import { useTenant } from '@/context/TenantContext'
import { useExtensions, SidebarItem } from '@/context/ExtensionContext'
import { useTranslations } from '@/context/I18nContext'

interface AppSidebarProps {
  securityAlertCount?: number
  activeExtensionItem?: SidebarItem | null
  onSelectExtension?: (item: SidebarItem) => void
}

interface NavItem {
  title: string
  url: string
  icon: any
  visible: boolean
  isExternal?: boolean
  badge?: number
}

export function AppSidebar({
  securityAlertCount = 0,
  activeExtensionItem,
  onSelectExtension,
}: AppSidebarProps) {
  const location = useLocation()
  const { user, hasPermission } = useAuth()
  const { currentCompany } = useTenant()
  const { uiComponents } = useExtensions()
  const t = useTranslations('Sidebar')

  const currentPath = location.pathname

  // Helper to determine if a route is active
  const isRouteActive = (href: string) => {
    if (href === '/') {
      return currentPath === '/'
    }
    return currentPath.startsWith(href)
  }

  // Permission filters
  const isAdmin = user?.role === 'SuperAdmin' || user?.role === 'Admin'
  const isSuperAdmin = user?.role === 'SuperAdmin'

  // Define nav groups
  const overviewGroup = [
    {
      title: t('dashboard'),
      url: '/',
      icon: LayoutDashboard,
      visible: isAdmin,
    },
  ]

  const fileGroup: NavItem[] = [
    {
      title: t('files'),
      url: '/files',
      icon: FileText,
      visible: hasPermission('files.view'),
    },
    {
      title: t('myPrivateFiles'),
      url: '/private-files',
      icon: Lock,
      visible: hasPermission('files.view'),
    },
    {
      title: t('requests'),
      url: '/file-requests',
      icon: Link2,
      visible: hasPermission('requests.view'),
    },
    {
      title: t('approvals'),
      url: '/approvals',
      icon: CheckCircle,
      visible: hasPermission('approvals.view') && ((currentCompany as any)?.approval_workflow_enabled ?? true),
    },
    {
      title: t('sharedWithMe'),
      url: '/shared-with-me',
      icon: Share2,
      visible: hasPermission('files.view'),
    },
    {
      title: t('myShares'),
      url: '/my-shares',
      icon: Send,
      visible: hasPermission('files.view'),
    },
    {
      title: t('apps'),
      url: '/apps',
      icon: Boxes,
      visible: true,
    },
    {
      title: t('firmwareFiles'),
      url: '/firmware-files',
      icon: Cpu,
      visible: hasPermission('files.view'),
    },
    {
      title: t('mqttClients'),
      url: '/mqtt-clients',
      icon: Radio,
      visible: hasPermission('files.view'),
    },
    {
      title: t('compilerRecords'),
      url: '/compiler-records',
      icon: History,
      visible: hasPermission('files.view'),
    },
  ]

  const adminGroup = [
    {
      title: t('companies'),
      url: '/companies',
      icon: Building2,
      visible: hasPermission('tenants.manage'),
    },
    {
      title: t('configs'),
      url: '/configs',
      icon: Sliders,
      visible: isAdmin,
    },
    {
      title: t('users'),
      url: '/users',
      icon: Users,
      visible: hasPermission('users.view'),
    },
    {
      title: t('security'),
      url: '/security',
      icon: Shield,
      visible: hasPermission('audit.view'),
      badge: securityAlertCount > 0 ? securityAlertCount : undefined,
    },
    {
      title: t('performance'),
      url: '/performance',
      icon: Activity,
      visible: isSuperAdmin,
    },
    {
      title: t('settings'),
      url: '/settings',
      icon: Settings,
      visible: hasPermission('settings.view'),
    },
  ]

  const { state } = useSidebar()

  return (
    <Sidebar collapsible='icon' variant='inset'>
      <SidebarHeader className='p-2'>
        {state === 'collapsed' ? (
          <div className='flex items-center justify-center w-full py-1'>
            <SidebarTrigger className='size-8 rounded-lg hover:bg-sidebar-accent text-sidebar-foreground/70 hover:text-sidebar-foreground transition-colors cursor-pointer' />
          </div>
        ) : (
          <div className='flex items-center justify-between gap-1 w-full'>
            <div className='flex-1 min-w-0'>
              <TeamSwitcher />
            </div>
            <SidebarTrigger className='shrink-0 size-8 rounded-lg hover:bg-sidebar-accent text-sidebar-foreground/70 hover:text-sidebar-foreground transition-colors cursor-pointer' />
          </div>
        )}
      </SidebarHeader>

      <SidebarContent>
        {/* Overview Group */}
        {overviewGroup.some((i) => i.visible) && (
          <SidebarGroup>
            <SidebarGroupLabel>{t('overview')}</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {overviewGroup
                  .filter((item) => item.visible)
                  .map((item) => {
                    const active = isRouteActive(item.url)
                    return (
                      <SidebarMenuItem key={item.url}>
                        <SidebarMenuButton
                          asChild
                          isActive={active}
                          tooltip={item.title}
                        >
                          <Link to={item.url}>
                            <item.icon className='size-4' />
                            <span>{item.title}</span>
                          </Link>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    )
                  })}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        )}

        {/* Files & Workspace Group */}
        {fileGroup.some((i) => i.visible) && (
          <SidebarGroup>
            <SidebarGroupLabel>{t('workspace')}</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {fileGroup
                  .filter((item) => item.visible)
                  .map((item) => {
                    const active = !item.isExternal && isRouteActive(item.url)
                    return (
                      <SidebarMenuItem key={item.url}>
                        <SidebarMenuButton
                          asChild
                          isActive={active}
                          tooltip={item.title}
                        >
                          {item.isExternal ? (
                            <a
                              href={item.url}
                              target='_blank'
                              rel='noopener noreferrer'
                            >
                              <item.icon className='size-4' />
                              <span>{item.title}</span>
                              <ExternalLink className='size-3 text-muted-foreground ml-auto group-data-[collapsible=icon]:hidden' />
                            </a>
                          ) : (
                            <Link to={item.url}>
                              <item.icon className='size-4' />
                              <span>{item.title}</span>
                            </Link>
                          )}
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    )
                  })}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        )}

        {/* Administration Group */}
        {adminGroup.some((i) => i.visible) && (
          <SidebarGroup>
            <SidebarGroupLabel>{t('administration')}</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {adminGroup
                  .filter((item) => item.visible)
                  .map((item) => {
                    const active = isRouteActive(item.url)
                    return (
                      <SidebarMenuItem key={item.url}>
                        <SidebarMenuButton
                          asChild
                          isActive={active}
                          tooltip={item.title}
                        >
                          <Link to={item.url}>
                            <item.icon className='size-4' />
                            <span>{item.title}</span>
                          </Link>
                        </SidebarMenuButton>
                        {item.badge !== undefined && (
                          <SidebarMenuBadge className='bg-destructive text-white rounded-full px-1.5 py-0.5 text-[10px] font-bold'>
                            {item.badge}
                          </SidebarMenuBadge>
                        )}
                      </SidebarMenuItem>
                    )
                  })}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        )}

        {/* Extension Sidebar Items */}
        {uiComponents.sidebar && uiComponents.sidebar.length > 0 && (
          <SidebarGroup>
            <SidebarGroupLabel>Extensions</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {uiComponents.sidebar.map((item) => {
                  const active = activeExtensionItem?.id === item.id
                  return (
                    <SidebarMenuItem key={item.id}>
                      <SidebarMenuButton
                        isActive={active}
                        tooltip={item.name}
                        onClick={() => onSelectExtension?.(item)}
                      >
                        {item.icon ? (
                          <img src={item.icon} alt="" className="size-4 shrink-0" />
                        ) : (
                          <Puzzle className="size-4" />
                        )}
                        <span>{item.name}</span>
                      </SidebarMenuButton>
                    </SidebarMenuItem>
                  )
                })}
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        )}
      </SidebarContent>

      <SidebarFooter>
        <NavUser />
      </SidebarFooter>
    </Sidebar>
  )
}
