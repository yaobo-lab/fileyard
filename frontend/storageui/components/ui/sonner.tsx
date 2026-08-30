"use client"

import { useTheme } from "next-themes"
import { Toaster as Sonner, type ToasterProps } from "sonner"

import {
  Alert01Icon,
  AppIcon,
  CancelCircleIcon,
  CheckmarkCircle01Icon,
  InformationCircleIcon,
  Loading03Icon,
} from "@/components/foundations/icons"

const Toaster = ({ ...props }: ToasterProps) => {
  const { theme = "system" } = useTheme()

  return (
    <Sonner
      theme={theme as ToasterProps["theme"]}
      className="toaster group"
      icons={{
        success: <AppIcon icon={CheckmarkCircle01Icon} className="size-4" />,
        info: <AppIcon icon={InformationCircleIcon} className="size-4" />,
        warning: <AppIcon icon={Alert01Icon} className="size-4" />,
        error: <AppIcon icon={CancelCircleIcon} className="size-4" />,
        loading: (
          <AppIcon icon={Loading03Icon} className="size-4 animate-spin" />
        ),
      }}
      style={
        {
          "--normal-bg": "var(--popover)",
          "--normal-text": "var(--popover-foreground)",
          "--normal-border": "var(--border)",
          "--border-radius": "var(--radius)",
        } as React.CSSProperties
      }
      {...props}
    />
  )
}

export { Toaster }
