import * as React from 'react'
import { ChevronsUpDown, Building2, Check, Settings } from 'lucide-react'
import { Link } from 'react-router-dom'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from '@/components/ui/sidebar'
import { useTenant } from '@/context/TenantContext'
import { useAuth } from '@/context/AuthContext'

export function TeamSwitcher() {
  const { isMobile } = useSidebar()
  const { currentCompany, companies, setCurrentCompany } = useTenant()
  const { hasPermission } = useAuth()

  // Default display if no company is selected
  const companyName = currentCompany?.name || 'Fileyard'
  const companyPlan = (currentCompany as any)?.plan || (currentCompany ? 'Organization' : 'Default Workspace')

  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <SidebarMenuButton
              size='lg'
              className='data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground'
            >
              <div className='flex aspect-square size-8 items-center justify-center rounded-lg bg-sidebar-primary text-sidebar-primary-foreground'>
                <Building2 className='size-4' />
              </div>
              <div className='grid flex-1 text-start text-sm leading-tight'>
                <span className='truncate font-semibold'>
                  {companyName}
                </span>
                <span className='truncate text-xs text-muted-foreground'>
                  {companyPlan}
                </span>
              </div>
              <ChevronsUpDown className='ms-auto size-4' />
            </SidebarMenuButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            className='w-[--radix-dropdown-menu-trigger-width] min-w-56 rounded-lg'
            align='start'
            side={isMobile ? 'bottom' : 'right'}
            sideOffset={4}
          >
            <DropdownMenuLabel className='text-xs text-muted-foreground'>
              Organizations & Tenants
            </DropdownMenuLabel>
            {companies && companies.length > 0 ? (
              companies.map((company) => {
                const isSelected = currentCompany?.id === company.id
                return (
                  <DropdownMenuItem
                    key={company.id}
                    onClick={() => setCurrentCompany(company)}
                    className='gap-2 p-2'
                  >
                    <div className='flex size-6 items-center justify-center rounded-sm border bg-background'>
                      <Building2 className='size-3.5 shrink-0 text-muted-foreground' />
                    </div>
                    <span className='flex-1 truncate text-sm font-medium'>
                      {company.name}
                    </span>
                    {isSelected && (
                      <Check className='size-4 text-primary' />
                    )}
                  </DropdownMenuItem>
                )
              })
            ) : (
              <div className='p-2 text-xs text-muted-foreground'>
                No other tenants available
              </div>
            )}
            {hasPermission('tenants.manage') && (
              <>
                <DropdownMenuSeparator />
                <DropdownMenuItem asChild className='gap-2 p-2'>
                  <Link to='/companies'>
                    <div className='flex size-6 items-center justify-center rounded-md border bg-background'>
                      <Settings className='size-3.5 text-muted-foreground' />
                    </div>
                    <div className='font-medium text-muted-foreground'>Manage Companies</div>
                  </Link>
                </DropdownMenuItem>
              </>
            )}
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
    </SidebarMenu>
  )
}
