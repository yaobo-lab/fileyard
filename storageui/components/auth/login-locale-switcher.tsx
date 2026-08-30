"use client"

import * as React from "react"
import { useRouter } from "next/navigation"
import { localeNames, locales, type Locale } from "@/i18n/config"
import { useLocale, useTranslations } from "next-intl"

import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { setLocale } from "@/app/actions/locale"

/** Language selector for the login page (no sidebar/settings available). */
export function LoginLocaleSwitcher() {
  const t = useTranslations("Login")
  const activeLocale = useLocale() as Locale
  const router = useRouter()
  const [isPending, startTransition] = React.useTransition()

  function changeLocale(next: Locale) {
    if (next === activeLocale) return
    startTransition(async () => {
      await setLocale(next)
      router.refresh()
    })
  }

  return (
    <Select
      value={activeLocale}
      onValueChange={(value) => changeLocale(value as Locale)}
      disabled={isPending}
    >
      <SelectTrigger
        size="sm"
        aria-label={t("language")}
        className="h-auto w-auto min-w-0 gap-1 border-none bg-transparent px-1 py-0 text-sm text-muted-foreground shadow-none before:hidden hover:text-foreground data-pressed:text-foreground dark:bg-transparent"
      >
        <SelectValue>{localeNames[activeLocale]}</SelectValue>
      </SelectTrigger>
      <SelectContent>
        {locales.map((locale) => (
          <SelectItem key={locale} value={locale}>
            {localeNames[locale]}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  )
}
