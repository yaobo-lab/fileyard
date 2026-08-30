import { cookies } from "next/headers"
import { defaultLocale, isLocale, LOCALE_COOKIE } from "@/i18n/config"
import { getRequestConfig } from "next-intl/server"

export default getRequestConfig(async () => {
  const cookieLocale = (await cookies()).get(LOCALE_COOKIE)?.value
  const locale = isLocale(cookieLocale) ? cookieLocale : defaultLocale

  return {
    locale,
    messages: (await import(`../messages/${locale}.json`)).default,
  }
})
