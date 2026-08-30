import type React from "react"

import { cn } from "@/lib/utils"
import { AppIcon, Loading03Icon } from "@/components/foundations/icons"

export function Spinner({
  className,
  ...props
}: Omit<React.ComponentProps<typeof AppIcon>, "icon">): React.ReactElement {
  return (
    <AppIcon
      aria-label="Loading"
      className={cn("animate-spin", className)}
      icon={Loading03Icon}
      role="status"
      {...props}
    />
  )
}
