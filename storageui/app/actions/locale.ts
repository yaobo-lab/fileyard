"use server"

import { cookies } from "next/headers"
import { isLocale, LOCALE_COOKIE, type Locale } from "@/i18n/config"

const ONE_YEAR_SECONDS = 60 * 60 * 24 * 365

/** Persist the chosen UI locale in a cookie; the next render picks it up. */
export async function setLocale(locale: Locale): Promise<void> {
  if (!isLocale(locale)) return
  ;(await cookies()).set(LOCALE_COOKIE, locale, {
    path: "/",
    maxAge: ONE_YEAR_SECONDS,
    sameSite: "lax",
  })
}
