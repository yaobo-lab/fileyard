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
  SidebarRail,
} from '@/components/ui/sidebar'
import { TeamSwitcher } from './TeamSwitcher'
import { NavUser } from './NavUser'
import { useAuth } from '@/context/AuthContext'
import { useTenant } from '@/context/TenantContext'
import { useExtensions, SidebarItem } from '@/context/ExtensionContext'

interface AppSidebarProps {
  securityAlertCount?: number
  activeExtensionItem?: SidebarItem | null
  onSelectExtension?: (item: SidebarItem) => void
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
      title: 'Dashboard',
      url: '/',
      icon: LayoutDashboard,
      visible: isAdmin,
    },
  ]

  const fileGroup = [
    {
      title: 'Files',
      url: '/files',
      icon: FileText,
      visible: hasPermission('files.view'),
    },
    {
      title: 'Requests',
      url: '/file-requests',
      icon: Link2,
      visible: hasPermission('requests.view'),
    },
    {
      title: 'Approvals',
      url: '/approvals',
      icon: CheckCircle,
      visible: hasPermission('approvals.view') && ((currentCompany as any)?.approval_workflow_enabled ?? true),
    },
    {
      title: 'Shared with me',
      url: '/shared-with-me',
      icon: Share2,
      visible: hasPermission('files.view'),
    },
    {
      title: 'Storage',
      url: '/storage',
      icon: Folder,
      visible: true,
      isExternal: true,
    },
  ]

  const adminGroup = [
    {
      title: 'Companies',
      url: '/companies',
      icon: Building2,
      visible: hasPermission('tenants.manage'),
    },
    {
      title: 'Users',
      url: '/users',
      icon: Users,
      visible: hasPermission('users.view'),
    },
    {
      title: 'Security',
      url: '/security',
      icon: Shield,
      visible: hasPermission('audit.view'),
      badge: securityAlertCount > 0 ? securityAlertCount : undefined,
    },
    {
      title: 'Performance',
      url: '/performance',
      icon: Activity,
      visible: isSuperAdmin,
    },
    {
      title: 'Settings',
      url: '/settings',
      icon: Settings,
      visible: hasPermission('settings.view'),
    },
  ]

  return (
    <Sidebar collapsible='icon' variant='inset'>
      <SidebarHeader>
        <TeamSwitcher />
      </SidebarHeader>

      <SidebarContent>
        {/* Overview Group */}
        {overviewGroup.some((i) => i.visible) && (
          <SidebarGroup>
            <SidebarGroupLabel>Overview</SidebarGroupLabel>
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
            <SidebarGroupLabel>Workspace</SidebarGroupLabel>
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
                              className='flex items-center justify-between w-full'
                            >
                              <div className='flex items-center gap-2'>
                                <item.icon className='size-4' />
                                <span>{item.title}</span>
                              </div>
                              <ExternalLink className='size-3 text-muted-foreground' />
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
            <SidebarGroupLabel>Administration</SidebarGroupLabel>
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
                          <Link to={item.url} className='flex items-center justify-between w-full'>
                            <div className='flex items-center gap-2'>
                              <item.icon className='size-4' />
                              <span>{item.title}</span>
                            </div>
                            {item.badge !== undefined && (
                              <SidebarMenuBadge className='bg-destructive text-white rounded-full px-1.5 py-0.5 text-[10px] font-bold'>
                                {item.badge}
                              </SidebarMenuBadge>
                            )}
                          </Link>
                        </SidebarMenuButton>
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
      <SidebarRail />
    </Sidebar>
  )
}
