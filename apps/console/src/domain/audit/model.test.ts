import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import {
	describeBudgetError,
	draftsOf,
	groupByDay,
	initials,
	monthWindow,
	runAgent,
	runDuration,
	saveRequestOf,
} from '#/domain/audit/model'

const run = (
	extra: Partial<Schemas.RunResponse> = {},
): Schemas.RunResponse => ({
	id: 'r1',
	status: 'succeeded',
	initiated_by: 'u',
	pipeline: [],
	created_at: '2026-11-18T10:00:00Z',
	...extra,
})

describe('runAgent', () => {
	it('reads the agent reference of a run', () => {
		const agent = {
			id: 'a1',
			name: 'Iris',
		} as unknown as Schemas.RunResponse['agent']
		expect(runAgent(run({ agent }))).toEqual({ id: 'a1', name: 'Iris' })
		expect(runAgent(run({ agent: null }))).toBeNull()
		expect(runAgent(run())).toBeNull()
	})
})

describe('runDuration', () => {
	it('formats seconds, minutes and hours', () => {
		const start = '2026-11-18T10:00:00Z'
		expect(
			runDuration(
				run({ started_at: start, finished_at: '2026-11-18T10:00:14Z' }),
			),
		).toBe('14 s')
		expect(
			runDuration(
				run({ started_at: start, finished_at: '2026-11-18T10:01:42Z' }),
			),
		).toBe('1 min 42')
		expect(
			runDuration(
				run({ started_at: start, finished_at: '2026-11-18T13:05:00Z' }),
			),
		).toBe('3 h 05')
		expect(runDuration(run())).toBe('–')
	})

	it('measures a running run against now', () => {
		const now = Date.parse('2026-11-18T10:00:30Z')
		expect(runDuration(run({ started_at: '2026-11-18T10:00:00Z' }), now)).toBe(
			'30 s',
		)
	})
})

describe('monthWindow', () => {
	it('spans the current UTC month', () => {
		expect(monthWindow(new Date('2026-12-18T10:00:00Z'))).toEqual({
			from: '2026-12-01T00:00:00.000Z',
			to: '2027-01-01T00:00:00.000Z',
		})
	})
})

describe('initials', () => {
	it('takes the first and last word', () => {
		expect(initials('Baptiste Parmantier')).toBe('BP')
		expect(initials('François-Guillaume de Ribreau')).toBe('FR')
		expect(initials('admin')).toBe('AD')
	})
})

describe('groupByDay', () => {
	it('keeps the order and groups consecutive days', () => {
		const event = (id: string, occurred_at: string) =>
			({ id, occurred_at }) as Schemas.AuditEventResponse
		const groups = groupByDay([
			event('1', '2026-11-18T12:00:00'),
			event('2', '2026-11-18T09:00:00'),
			event('3', '2026-11-17T09:00:00'),
		])
		expect(groups.map((g) => g.events.map((e) => e.id))).toEqual([
			['1', '2'],
			['3'],
		])
	})
})

describe('budgets', () => {
	const budgets: Schemas.BudgetResponse[] = [
		{
			scope: 'organization',
			monthly_limit_cents: 40_000,
			alert_threshold_percent: 80,
			spent_micros: 0,
			percent: 0,
			alert: false,
			exceeded: false,
			updated_by: 'x',
			updated_at: 'x',
		},
		{
			scope: 'agent',
			agent_id: 'a1',
			agent_name: 'Iris',
			monthly_limit_cents: 1_050,
			alert_threshold_percent: 90,
			spent_micros: 0,
			percent: 0,
			alert: false,
			exceeded: false,
			updated_by: 'x',
			updated_at: 'x',
		},
	]

	it('round-trips the drafts into a full replacement request', () => {
		const drafts = draftsOf(budgets)
		expect(drafts[1]).toEqual({
			agentId: 'a1',
			label: 'Iris',
			limit: '10.5',
			threshold: '90',
		})
		expect(saveRequestOf(drafts)).toEqual({
			ok: true,
			request: {
				organization: {
					monthly_limit_cents: 40_000,
					alert_threshold_percent: 80,
				},
				agents: [
					{
						agent_id: 'a1',
						monthly_limit_cents: 1_050,
						alert_threshold_percent: 90,
					},
				],
			},
		})
	})

	it('sends a null organization when its budget is removed', () => {
		const result = saveRequestOf(draftsOf(budgets).slice(1))
		expect(result.ok && result.request.organization).toBeNull()
	})

	it('accepts a comma and omits an empty threshold', () => {
		const result = saveRequestOf([
			{ agentId: null, label: 'Organisation', limit: '12,5', threshold: '' },
		])
		expect(result).toEqual({
			ok: true,
			request: { organization: { monthly_limit_cents: 1_250 }, agents: [] },
		})
	})

	it('rejects invalid amounts and thresholds', () => {
		expect(
			saveRequestOf([
				{ agentId: null, label: 'Organisation', limit: 'abc', threshold: '80' },
			]),
		).toEqual({
			ok: false,
			reason: 'Montant invalide pour Organisation.',
		})
		expect(
			saveRequestOf([
				{ agentId: 'a1', label: 'Iris', limit: '10', threshold: '150' },
			]).ok,
		).toBe(false)
	})

	it('explains a forbidden save', () => {
		expect(describeBudgetError({ status: 403 })).toContain('administrateurs')
		expect(describeBudgetError(new Error('x'))).toBe(
			"Les budgets n'ont pas pu être enregistrés.",
		)
	})
})
