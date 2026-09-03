import * as React from 'react'
import { useLocation } from 'react-router-dom'
import { Search, Sun, Moon, Keyboard, ShieldAlert } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Separator } from '@/components/ui/separator'
import { SidebarTrigger } from '@/components/ui/sidebar'
import { Button } from '@/components/ui/button'
import { NotificationBell } from '@/components/NotificationBell'
import { useTheme } from '@/context/ThemeContext'
import { useKeyboardShortcutsContext } from '@/context/KeyboardShortcutsContext'
import { useSettings } from '@/context/SettingsContext'

interface HeaderProps extends React.HTMLAttributes<HTMLElement> {
  onSearchClick?: () => void
  searchPlaceholder?: string
}

// Route title dictionary
const ROUTE_TITLES: Record<string, string> = {
  '/': 'Dashboard',
  '/files': 'Files',
  '/file-requests': 'File Requests',
  '/approvals': 'Approvals',
  '/shared-with-me': 'Shared with Me',
  '/companies': 'Companies',
  '/users': 'Users',
  '/roles': 'Roles & Permissions',
  '/audit-logs': 'Audit Logs',
  '/security': 'Security Center',
  '/performance': 'Performance',
  '/settings': 'Settings',
  '/settings/general': 'General Settings',
  '/settings/branding': 'Branding',
  '/settings/pages': 'Page Settings',
  '/settings/email-templates': 'Email Templates',
  '/settings/shortcuts': 'Shortcuts',
  '/settings/system': 'System Settings',
  '/settings/virus-scan': 'Virus Scanning',
  '/settings/sso': 'SSO Integration',
  '/settings/backup': 'Backup & Restore',
  '/settings/admin': 'Admin Settings',
  '/profile': 'My Profile',
  '/notifications': 'Notifications',
  '/recycle-bin': 'Recycle Bin',
  '/help': 'Help & Documentation',
  '/quickstart': 'Quickstart',
}

export function Header({
  className,
  onSearchClick,
  searchPlaceholder = 'Search files...',
  ...props
}: HeaderProps) {
  const location = useLocation()
  const { theme, toggleTheme } = useTheme()
  const { toggleHelp } = useKeyboardShortcutsContext()
  const { complianceMode } = useSettings()

  const currentTitle = ROUTE_TITLES[location.pathname] || 'Workspace'

  return (
    <header
      className={cn(
        'sticky top-0 z-40 flex h-14 shrink-0 items-center justify-between gap-2 border-b bg-background/95 px-4 backdrop-blur-sm transition-all',
        className
      )}
      {...props}
    >
      <div className='flex items-center gap-2 sm:gap-3'>
        <SidebarTrigger className='-ml-1' />
        <Separator orientation='vertical' className='h-4' />
        <span className='font-semibold text-sm sm:text-base tracking-tight truncate max-w-[200px] sm:max-w-none'>
          {currentTitle}
        </span>
      </div>

      <div className='flex items-center gap-2'>
        {/* Global Search Button / Trigger */}
        <button
          type='button'
          onClick={onSearchClick}
          className='flex items-center gap-2 h-8 w-40 sm:w-64 rounded-md border border-input bg-background/50 px-3 text-xs text-muted-foreground shadow-xs transition-colors hover:bg-accent hover:text-accent-foreground cursor-pointer text-start'
        >
          <Search className='size-3.5 shrink-0 text-muted-foreground' />
          <span className='truncate flex-1'>{searchPlaceholder}</span>
          <kbd className='pointer-events-none hidden sm:inline-flex h-4 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground'>
            ⌘K
          </kbd>
        </button>

        {/* Compliance Mode Indicator if active */}
        {complianceMode && (
          <div className='hidden md:flex items-center gap-1 px-2 py-0.5 rounded text-xs bg-amber-500/10 text-amber-600 dark:text-amber-400 font-medium border border-amber-500/20'>
            <ShieldAlert className='size-3' />
            <span>Compliance Mode</span>
          </div>
        )}

        {/* Keyboard Shortcuts Trigger */}
        <Button
          variant='ghost'
          size='icon'
          className='h-8 w-8 text-muted-foreground hover:text-foreground'
          onClick={toggleHelp}
          title='Keyboard Shortcuts (?)'
        >
          <Keyboard className='size-4' />
        </Button>

        {/* Dark / Light Theme Toggle */}
        <Button
          variant='ghost'
          size='icon'
          className='h-8 w-8 text-muted-foreground hover:text-foreground'
          onClick={toggleTheme}
          title={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}
        >
          {theme === 'dark' ? (
            <Sun className='size-4 text-amber-400 transition-transform' />
          ) : (
            <Moon className='size-4 text-slate-700 transition-transform' />
          )}
        </Button>

        <Separator orientation='vertical' className='h-4 hidden sm:block' />

        {/* Notifications Bell */}
        <NotificationBell />
      </div>
    </header>
  )
}
