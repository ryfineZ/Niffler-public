import { describe, expect, it } from 'vitest'
import { applyUsageActiveRequestUpdate } from '../activeRequestUpdates'
import type { UsageRecord } from '../../types'

function buildRecord(overrides: Partial<UsageRecord> = {}): UsageRecord {
  return {
    id: 'usage-1',
    model: 'gpt-5.5',
    input_tokens: 0,
    output_tokens: 0,
    total_tokens: 0,
    cost: 0,
    is_stream: true,
    status: 'streaming',
    created_at: '2026-06-04T10:13:56+08:00',
    ...overrides,
  }
}

describe('applyUsageActiveRequestUpdate', () => {
  it('replaces stale charge breakdown and clears stale fallback flags', () => {
    const record = buildRecord({
      has_fallback: true,
      charge_breakdown: {
        official_cost: 0,
        package_debit: 0,
        wallet_debit: 0,
        user_debit: 0,
      },
    })

    applyUsageActiveRequestUpdate(record, {
      id: 'usage-1',
      status: 'completed',
      input_tokens: 9287,
      effective_input_tokens: 9287,
      output_tokens: 364,
      cache_read_input_tokens: 15872,
      official_cost: 0.065291,
      cost: 0.00326455,
      actual_cost: 0.00065291,
      rate_multiplier: 0.01,
      sales_multiplier: 0.05,
      has_fallback: false,
      charge_breakdown: {
        official_cost: 0.065291,
        package_debit: 0,
        wallet_debit: 0.00326455,
        wallet_multiplier: 0.05,
        user_debit: 0.00326455,
      },
    })

    expect(record.status).toBe('completed')
    expect(record.has_fallback).toBe(false)
    expect(record.charge_breakdown?.wallet_debit).toBe(0.00326455)
    expect(record.charge_breakdown?.user_debit).toBe(0.00326455)
  })

  it('does not replace a known provider with an unknown active update value', () => {
    const record = buildRecord({ provider: 'https://c-api.cc/' })

    applyUsageActiveRequestUpdate(record, {
      id: 'usage-1',
      status: 'streaming',
      input_tokens: 1,
      output_tokens: 0,
      cost: 0,
      provider: 'unknown',
    })

    expect(record.provider).toBe('https://c-api.cc/')
  })

  it('applies provider transfer routes from active updates', () => {
    const record = buildRecord({ provider: 'cc-max(zzshu)1.0' })

    applyUsageActiveRequestUpdate(record, {
      id: 'usage-1',
      provider_route: ['cc-max(link)1.0', 'cc-max(zzshu)1.0'],
    })

    expect(record.provider_route).toEqual(['cc-max(link)1.0', 'cc-max(zzshu)1.0'])
  })
})
