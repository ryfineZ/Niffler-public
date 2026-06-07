import type { BillingPlan } from '@/api/billing'

function pad2(value: number): string {
  return String(value).padStart(2, '0')
}

export function formatDatetimeLocal(date: Date): string {
  return [
    date.getFullYear(),
    '-',
    pad2(date.getMonth() + 1),
    '-',
    pad2(date.getDate()),
    'T',
    pad2(date.getHours()),
    ':',
    pad2(date.getMinutes()),
  ].join('')
}

export function parseDatetimeLocal(value: string): Date | null {
  const trimmed = value.trim()
  if (!trimmed) return null
  const date = new Date(trimmed)
  if (Number.isNaN(date.getTime())) return null
  return date
}

export function datetimeLocalToIso(value: string): string | null | undefined {
  const trimmed = value.trim()
  if (!trimmed) return null
  const date = parseDatetimeLocal(trimmed)
  if (!date) return undefined
  return date.toISOString()
}

export function addPlanDuration(start: Date, plan: Pick<BillingPlan, 'duration_unit' | 'duration_value'>): Date {
  const durationValue = Math.max(1, Number(plan.duration_value || 1))
  const expiresAt = new Date(start.getTime())
  switch (plan.duration_unit) {
    case 'month':
      expiresAt.setMonth(expiresAt.getMonth() + durationValue)
      return expiresAt
    case 'year':
      expiresAt.setFullYear(expiresAt.getFullYear() + durationValue)
      return expiresAt
    case 'day':
    case 'custom':
    default:
      expiresAt.setDate(expiresAt.getDate() + durationValue)
      return expiresAt
  }
}

export function defaultGrantPlanTimeWindow(plan: Pick<BillingPlan, 'duration_unit' | 'duration_value'>, now = new Date()) {
  const startsAt = new Date(now.getTime())
  const expiresAt = addPlanDuration(startsAt, plan)
  return {
    startsAt: formatDatetimeLocal(startsAt),
    expiresAt: formatDatetimeLocal(expiresAt),
  }
}
