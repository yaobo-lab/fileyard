"use client"

import * as React from "react"
import { useLocale, useTranslations } from "next-intl"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogPanel,
  DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import {
  AppIcon,
  ArrowLeft01Icon,
  ArrowRight01Icon,
  Calendar03Icon,
} from "@/components/foundations/icons"

export function formatDateInputValue(date: Date | undefined) {
  if (!date) return ""

  const pad = (value: number) => String(value).padStart(2, "0")

  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
}

export function parseDateInputValue(value: string) {
  const trimmed = value.trim()

  if (!trimmed) return undefined

  const isoMatch = /^(\d{4})-(\d{1,2})-(\d{1,2})$/.exec(trimmed)

  if (isoMatch) {
    const date = new Date(
      Number(isoMatch[1]),
      Number(isoMatch[2]) - 1,
      Number(isoMatch[3])
    )

    return Number.isNaN(date.getTime()) ? undefined : date
  }

  const parsed = Date.parse(trimmed)

  return Number.isNaN(parsed) ? undefined : new Date(parsed)
}

export const DATE_RANGE_DIALOG_PRESETS = [
  "Last 7 days",
  "This month",
  "Last 1 month",
  "Last 3 months",
  "This year",
  "Last 12 months",
]

// Stable preset id (English, also the switch key) → catalog key for display.
const PRESET_LABEL_KEYS: Record<string, string> = {
  "Last 7 days": "rangeLast7",
  "This month": "rangeThisMonth",
  "Last 1 month": "rangeLastMonth",
  "Last 3 months": "rangeLast3Months",
  "This year": "rangeThisYear",
  "Last 12 months": "rangeLast12Months",
}

export function dateRangePresetRange(preset: string) {
  const from = new Date()
  const to = new Date()

  from.setHours(0, 0, 0, 0)
  to.setHours(23, 59, 59, 999)

  switch (preset) {
    case "Last 7 days":
      from.setDate(from.getDate() - 6)
      break
    case "This month":
      from.setDate(1)
      break
    case "Last 1 month":
      from.setMonth(from.getMonth() - 1)
      break
    case "Last 3 months":
      from.setMonth(from.getMonth() - 3)
      break
    case "This year":
      from.setMonth(0, 1)
      break
    case "Last 12 months":
      from.setFullYear(from.getFullYear() - 1)
      break
  }
  return { from, to }
}

export const WEEKDAY_LABELS = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]

export function calendarDayKey(date: Date) {
  return date.getFullYear() * 10_000 + date.getMonth() * 100 + date.getDate()
}

// Two-month range calendar for the custom date range dialog (one month at
// phone widths). Clicking sets the start, then the end; clicking before the
// start swaps the ends, and a third click restarts the range.
export function FileSystemRangeCalendar({
  onSelectAction,
  range,
}: {
  onSelectAction: (range: { from?: Date; to?: Date }) => void
  range: { from?: Date; to?: Date }
}) {
  const t = useTranslations("Dialogs")
  const locale = useLocale()
  // Localized short weekday names, Sunday-first (2024-01-07 is a Sunday).
  const weekdayLabels = React.useMemo(() => {
    const formatter = new Intl.DateTimeFormat(locale, { weekday: "short" })
    return Array.from({ length: 7 }, (_, index) =>
      formatter.format(new Date(2024, 0, 7 + index))
    )
  }, [locale])
  const [viewMonth, setViewMonth] = React.useState(() => {
    const base = range.from ?? new Date()

    return new Date(base.getFullYear(), base.getMonth(), 1)
  })
  const months = [
    viewMonth,
    new Date(viewMonth.getFullYear(), viewMonth.getMonth() + 1, 1),
  ]
  const fromKey = range.from ? calendarDayKey(range.from) : null
  const toKey = range.to ? calendarDayKey(range.to) : null
  const todayKey = calendarDayKey(new Date())

  const handleDayClick = (day: Date) => {
    if (!range.from || range.to) {
      onSelectAction({ from: day })
    } else if (calendarDayKey(day) < calendarDayKey(range.from)) {
      onSelectAction({ from: day, to: range.from })
    } else {
      onSelectAction({ from: range.from, to: day })
    }
  }

  return (
    <div className="relative">
      <button
        type="button"
        aria-label={t("previousMonth")}
        onClick={() =>
          setViewMonth(
            (previous) =>
              new Date(previous.getFullYear(), previous.getMonth() - 1, 1)
          )
        }
        className="absolute top-0 left-0 flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors outline-none hover:bg-accent hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
      >
        <AppIcon icon={ArrowLeft01Icon} className="size-4" />
      </button>
      <button
        type="button"
        aria-label={t("nextMonth")}
        onClick={() =>
          setViewMonth(
            (previous) =>
              new Date(previous.getFullYear(), previous.getMonth() + 1, 1)
          )
        }
        className="absolute top-0 right-0 flex size-6 items-center justify-center rounded-md text-muted-foreground transition-colors outline-none hover:bg-accent hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring"
      >
        <AppIcon icon={ArrowRight01Icon} className="size-4" />
      </button>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        {months.map((month, monthIndex) => {
          const firstWeekday = month.getDay()
          const dayCount = new Date(
            month.getFullYear(),
            month.getMonth() + 1,
            0
          ).getDate()
          const cells = [
            ...Array.from({ length: firstWeekday }, () => null),
            ...Array.from(
              { length: dayCount },
              (_, index) =>
                new Date(month.getFullYear(), month.getMonth(), index + 1)
            ),
          ]

          return (
            <div
              key={`${month.getFullYear()}-${month.getMonth()}`}
              className={cn(monthIndex === 1 && "max-sm:hidden")}
            >
              <div className="text-center text-sm leading-6 font-medium">
                {month.toLocaleDateString(locale, {
                  month: "long",
                  year: "numeric",
                })}
              </div>
              <div className="mt-1 grid grid-cols-7 text-center text-xs text-muted-foreground">
                {weekdayLabels.map((weekday, weekdayIndex) => (
                  <span key={weekdayIndex} className="h-6 leading-6">
                    {weekday}
                  </span>
                ))}
              </div>
              <div className="grid grid-cols-7 gap-y-px">
                {cells.map((day, cellIndex) => {
                  if (!day) return <span key={cellIndex} />

                  const dayKey = calendarDayKey(day)
                  const isFrom = dayKey === fromKey
                  const isTo = dayKey === toKey
                  const isWithinRange =
                    fromKey !== null &&
                    toKey !== null &&
                    dayKey > fromKey &&
                    dayKey < toKey

                  return (
                    <button
                      key={cellIndex}
                      type="button"
                      onClick={() => handleDayClick(day)}
                      className={cn(
                        "flex h-7 items-center justify-center rounded-md text-xs tabular-nums transition-colors outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring",
                        isWithinRange && "rounded-none bg-accent",
                        (isFrom || isTo) &&
                          "bg-primary text-primary-foreground hover:bg-primary",
                        isFrom &&
                          toKey !== null &&
                          fromKey !== toKey &&
                          "rounded-r-none",
                        isTo && fromKey !== toKey && "rounded-l-none",
                        dayKey === todayKey &&
                          !isFrom &&
                          !isTo &&
                          "font-semibold text-primary"
                      )}
                    >
                      {day.getDate()}
                    </button>
                  )
                })}
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}

// Custom date range dialog mirroring Extend's table filters: From/To inputs,
// a two-month range calendar, and quick presets. Applied ranges span from
// the start of the first day to the end of the last.
export function FileSystemDateRangeDialog({
  initialRange,
  onApplyAction,
  onCloseAction,
}: {
  initialRange?: { from: Date; to: Date }
  onApplyAction: (from: Date, to: Date) => void
  onCloseAction: () => void
}) {
  const t = useTranslations("Dialogs")
  const tc = useTranslations("Common")
  const [range, setRange] = React.useState<{ from?: Date; to?: Date }>(
    () => initialRange ?? {}
  )
  const [fromInput, setFromInput] = React.useState(() =>
    formatDateInputValue(initialRange?.from)
  )
  const [toInput, setToInput] = React.useState(() =>
    formatDateInputValue(initialRange?.to)
  )

  const selectRange = (next: { from?: Date; to?: Date }) => {
    setRange(next)
    if (next.from) setFromInput(formatDateInputValue(next.from))
    if (next.to) setToInput(formatDateInputValue(next.to))
  }

  const dateField = (
    label: string,
    value: string,
    onChange: (value: string) => void
  ) => (
    <div className="flex flex-1 flex-col gap-1.5">
      <span className="text-xs font-medium">{label}</span>
      <div className="relative flex items-center">
        <AppIcon
          icon={Calendar03Icon}
          className="pointer-events-none absolute left-2.5 size-3.5 text-muted-foreground"
        />
        <Input
          type="text"
          value={value}
          placeholder="YYYY-MM-DD"
          aria-label={t("dateAria", { label })}
          onChange={(event) => onChange(event.target.value)}
          className="h-8 pl-8 sm:h-8"
        />
      </div>
    </div>
  )

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onCloseAction()
      }}
    >
      <DialogContent className="w-120 max-w-[calc(100vw-2rem)]">
        <DialogHeader>
          <DialogTitle>{t("dateRangeTitle")}</DialogTitle>
        </DialogHeader>
        <DialogPanel className="flex flex-col gap-4">
          <div className="flex gap-3">
            {dateField(t("from"), fromInput, (value) => {
              setFromInput(value)

              const parsed = parseDateInputValue(value)

              if (parsed)
                setRange((previous) => ({ ...previous, from: parsed }))
            })}
            {dateField(t("to"), toInput, (value) => {
              setToInput(value)

              const parsed = parseDateInputValue(value)

              if (parsed) setRange((previous) => ({ ...previous, to: parsed }))
            })}
          </div>
          <FileSystemRangeCalendar range={range} onSelectAction={selectRange} />
          <div className="grid grid-cols-3 gap-2">
            {DATE_RANGE_DIALOG_PRESETS.map((preset) => (
              <Button
                key={preset}
                type="button"
                variant="outline"
                size="sm"
                onClick={() => selectRange(dateRangePresetRange(preset))}
              >
                {t(PRESET_LABEL_KEYS[preset])}
              </Button>
            ))}
          </div>
        </DialogPanel>
        <DialogFooter>
          <Button type="button" variant="outline" onClick={onCloseAction}>
            {tc("cancel")}
          </Button>
          <Button
            type="button"
            disabled={!range.from || !range.to}
            onClick={() => {
              if (!range.from || !range.to) return

              const from = new Date(range.from)
              const to = new Date(range.to)

              from.setHours(0, 0, 0, 0)
              to.setHours(23, 59, 59, 999)
              onApplyAction(from, to)
            }}
          >
            {tc("apply")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
