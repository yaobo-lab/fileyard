import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar"
import { AppSidebar } from "@/components/app-sidebar"
import { AddConnectionDialog } from "@/components/connections/add-connection-dialog"
import { ConnectionStoreHydrator } from "@/components/connections/connection-store-hydrator"

export default function AppLayout({ children }: { children: React.ReactNode }) {
  return (
    <ConnectionStoreHydrator>
      <SidebarProvider
        data-slot="layout"
        className="h-svh overflow-hidden bg-background"
      >
        <AppSidebar />
        <SidebarInset className="min-h-0 overflow-hidden">
          {children}
        </SidebarInset>
      </SidebarProvider>
      <AddConnectionDialog />
    </ConnectionStoreHydrator>
  )
}
