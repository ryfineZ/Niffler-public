import { describe, expect, it } from 'vitest'
import { addPlanDuration, datetimeLocalToIso, defaultGrantPlanTimeWindow } from '../grantPlanTime'

describe('grantPlanTime', () => {
  it('defaults start to current local time and expiry to plan duration', () => {
    const now = new Date(2026, 4, 29, 17, 37, 30)
    const window = defaultGrantPlanTimeWindow({ duration_unit: 'month', duration_value: 1 }, now)

    expect(window.startsAt).toBe('2026-05-29T17:37')
    expect(window.expiresAt).toBe('2026-06-29T17:37')
    expect(addPlanDuration(new Date(window.startsAt), { duration_unit: 'month', duration_value: 1 }).getTime())
      .toBe(new Date(window.expiresAt).getTime())
  })

  it('keeps manual datetime values valid for API submission', () => {
    const iso = datetimeLocalToIso('2026-05-11T17:37')

    expect(iso).toMatch(/^2026-05-11T/)
  })
})
