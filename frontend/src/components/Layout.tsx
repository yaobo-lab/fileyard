import * as React from 'react'
import { Outlet, useNavigate } from 'react-router-dom'
import { SidebarProvider, SidebarInset } from '@/components/ui/sidebar'
import { AppSidebar } from '@/components/layout/AppSidebar'
import { Header } from '@/components/layout/Header'
import { Main } from '@/components/layout/Main'
import { SearchDialog } from '@/components/layout/SearchDialog'
import { ExtensionPanel } from '@/components/ExtensionPanel'
import { useAuth, useAuthFetch } from '@/context/AuthContext'
import { useTheme } from '@/context/ThemeContext'
import { useKeyboardShortcutsContext } from '@/context/KeyboardShortcutsContext'
import { useKeyboardShortcuts, Shortcut } from '@/hooks/useKeyboardShortcuts'
import { ShortcutActionId } from '@/hooks/shortcutPresets'
import { SidebarItem } from '@/context/ExtensionContext'
import { getCookie } from '@/lib/cookies'

export function Layout() {
  const { user, hasPermission } = useAuth()
  const authFetch = useAuthFetch()
  const { toggleTheme } = useTheme()
  const { toggleHelp, isHelpOpen, getResolvedBinding } = useKeyboardShortcutsContext()
  const navigate = useNavigate()

  const [securityAlertCount, setSecurityAlertCount] = React.useState(0)
  const [activeExtensionItem, setActiveExtensionItem] = React.useState<SidebarItem | null>(null)
  const [isSearchOpen, setIsSearchOpen] = React.useState(false)

  // Dynamic search placeholder based on role
  const getSearchPlaceholder = () => {
    switch (user?.role) {
      case 'SuperAdmin':
        return 'Search companies, users, or files...'
      case 'Admin':
      case 'Manager':
        return 'Search users or files...'
      default:
        return 'Search files...'
    }
  }

  // Keyboard shortcut helper
  const getBinding = React.useCallback(
    (actionId: ShortcutActionId) => {
      const binding = getResolvedBinding(actionId)
      return binding ? { keys: binding.keys, isSequence: binding.isSequence } : null
    },
    [getResolvedBinding]
  )

  // Configure global keyboard shortcuts
  const shortcuts = React.useMemo(() => {
    const allShortcuts: Shortcut[] = []

    const navDashboard = getBinding('nav.dashboard')
    if (navDashboard) {
      allShortcuts.push({
        id: 'nav.dashboard',
        keys: navDashboard.keys,
        description: 'Go to Dashboard',
        category: 'navigation',
        action: () => navigate('/'),
        isSequence: navDashboard.isSequence,
        enabled: user?.role === 'SuperAdmin' || user?.role === 'Admin',
      })
    }

    const navFiles = getBinding('nav.files')
    if (navFiles) {
      allShortcuts.push({
        id: 'nav.files',
        keys: navFiles.keys,
        description: 'Go to Files',
        category: 'navigation',
        action: () => navigate('/files'),
        isSequence: navFiles.isSequence,
        enabled: hasPermission('files.view'),
      })
    }

    const navUsers = getBinding('nav.users')
    if (navUsers) {
      allShortcuts.push({
        id: 'nav.users',
        keys: navUsers.keys,
        description: 'Go to Users',
        category: 'navigation',
        action: () => navigate('/users'),
        isSequence: navUsers.isSequence,
        enabled: hasPermission('users.view'),
      })
    }

    const navCompanies = getBinding('nav.companies')
    if (navCompanies) {
      allShortcuts.push({
        id: 'nav.companies',
        keys: navCompanies.keys,
        description: 'Go to Companies',
        category: 'navigation',
        action: () => navigate('/companies'),
        isSequence: navCompanies.isSequence,
        enabled: hasPermission('tenants.manage'),
      })
    }

    const navSettings = getBinding('nav.settings')
    if (navSettings) {
      allShortcuts.push({
        id: 'nav.settings',
        keys: navSettings.keys,
        description: 'Go to Settings',
        category: 'navigation',
        action: () => navigate('/settings'),
        isSequence: navSettings.isSequence,
        enabled: hasPermission('settings.view'),
      })
    }

    const navProfile = getBinding('nav.profile')
    if (navProfile) {
      allShortcuts.push({
        id: 'nav.profile',
        keys: navProfile.keys,
        description: 'Go to Profile',
        category: 'navigation',
        action: () => navigate('/profile'),
        isSequence: navProfile.isSequence,
      })
    }

    const navNotifications = getBinding('nav.notifications')
    if (navNotifications) {
      allShortcuts.push({
        id: 'nav.notifications',
        keys: navNotifications.keys,
        description: 'Go to Notifications',
        category: 'navigation',
        action: () => navigate('/notifications'),
        isSequence: navNotifications.isSequence,
      })
    }

    // UI control shortcuts
    const uiSearch = getBinding('ui.search')
    if (uiSearch) {
      allShortcuts.push({
        id: 'ui.search',
        keys: uiSearch.keys,
        description: 'Focus search',
        category: 'ui',
        action: () => setIsSearchOpen(true),
        isSequence: uiSearch.isSequence,
      })
    }

    const uiTheme = getBinding('ui.theme')
    if (uiTheme) {
      allShortcuts.push({
        id: 'ui.theme',
        keys: uiTheme.keys,
        description: 'Toggle dark/light theme',
        category: 'ui',
        action: () => toggleTheme(),
        isSequence: uiTheme.isSequence,
      })
    }

    const uiHelp = getBinding('ui.help')
    if (uiHelp) {
      allShortcuts.push({
        id: 'ui.help',
        keys: uiHelp.keys,
        description: 'Show keyboard shortcuts',
        category: 'ui',
        action: () => toggleHelp(),
        isSequence: uiHelp.isSequence,
      })
    }

    const uiClose = getBinding('ui.close')
    if (uiClose) {
      allShortcuts.push({
        id: 'ui.close',
        keys: uiClose.keys,
        description: 'Close modal or panel',
        category: 'ui',
        action: () => {
          if (isHelpOpen) return
          setIsSearchOpen(false)
          setActiveExtensionItem(null)
        },
        isSequence: uiClose.isSequence,
      })
    }

    return allShortcuts
  }, [navigate, hasPermission, user, toggleTheme, toggleHelp, isHelpOpen, getBinding])

  useKeyboardShortcuts(shortcuts)

  // Fetch security alert badge count
  React.useEffect(() => {
    const fetchSecurityBadge = async () => {
      if (!user || !hasPermission('audit.view')) return
      try {
        const response = await authFetch('/api/security/alerts/badge')
        if (response.ok) {
          const data = await response.json()
          setSecurityAlertCount(data.count || 0)
        }
      } catch (error) {
        console.error('Failed to fetch security badge:', error)
      }
    }
    fetchSecurityBadge()
    const interval = setInterval(fetchSecurityBadge, 5 * 60 * 1000)
    return () => clearInterval(interval)
  }, [user, authFetch, hasPermission])

  const defaultSidebarOpen = getCookie('sidebar_state') !== 'false'

  return (
    <SidebarProvider defaultOpen={defaultSidebarOpen}>
      <AppSidebar
        securityAlertCount={securityAlertCount}
        activeExtensionItem={activeExtensionItem}
        onSelectExtension={(item) => setActiveExtensionItem(item)}
      />

      <SidebarInset className='flex flex-col min-h-svh bg-background'>
        <Header
          onSearchClick={() => setIsSearchOpen(true)}
          searchPlaceholder={getSearchPlaceholder()}
        />

        <SearchDialog
          open={isSearchOpen}
          onOpenChange={setIsSearchOpen}
          placeholder={getSearchPlaceholder()}
        />

        {/* Optional extension panel */}
        {activeExtensionItem && (
          <ExtensionPanel
            item={activeExtensionItem}
            onClose={() => setActiveExtensionItem(null)}
          />
        )}

        <Main>
          <React.Suspense fallback={null}>
            <Outlet />
          </React.Suspense>
        </Main>
      </SidebarInset>
    </SidebarProvider>
  )
}
