import { describe, expect, it } from 'vitest'
import {
	createBody,
	describeRoutineError,
	draftOf,
	emptyDraft,
	filterRoutines,
	relativeTime,
	runDuration,
	runOutcome,
	timezoneOptions,
	triggersSummary,
	updateBody,
	validateDraft,
} from '#/domain/routines/model'
import { routine, routineRun, trigger } from '#/domain/routines/test-fixtures'
import { ApiRequestError } from '#/lib/api-error'

const valid = () => ({
	...emptyDraft(),
	name: 'Revue hebdo',
	prompt: 'Faire le point',
	agentId: 'a-dev',
})

describe('validateDraft', () => {
	it('requires a name, a prompt and an agent', () => {
		expect(validateDraft(emptyDraft())).toMatch(/nom/)
		expect(validateDraft({ ...valid(), prompt: ' ' })).toMatch(/consigne/)
		expect(validateDraft({ ...valid(), agentId: '' })).toMatch(/agent/)
		expect(validateDraft(valid())).toBeNull()
	})

	it('checks the slug and the schedule only when planned', () => {
		expect(validateDraft({ ...valid(), slug: 'Pas Bon' })).toMatch(/kebab/)
		const noDays = {
			...valid(),
			schedule: { ...valid().schedule, days: [] },
		}
		expect(validateDraft(noDays)).toMatch(/au moins un jour/)
		expect(validateDraft({ ...noDays, planned: false })).toBeNull()
	})
})

describe('createBody', () => {
	it('maps the draft, with the schedule and without an empty slug', () => {
		expect(createBody({ ...valid(), allowWrites: true })).toEqual({
			name: 'Revue hebdo',
			prompt: 'Faire le point',
			agent_id: 'a-dev',
			project_id: null,
			priority: 'normal',
			title_template: '{nom} · {date}',
			timezone: 'Europe/Paris',
			enabled: true,
			if_open: 'skip',
			if_missed: 'run_once',
			allow_writes: true,
			schedule: { frequency: 'weekly', time: '08:00', days: ['mon'] },
		})
		expect(createBody({ ...valid(), planned: false }).schedule).toBeNull()
		expect(createBody({ ...valid(), slug: 'revue' }).slug).toBe('revue')
	})
})

describe('updateBody', () => {
	it('sends only the changed fields', () => {
		const current = routine('r1', { project_id: 'p1' })
		expect(updateBody(current, draftOf(current))).toEqual({})
		expect(
			updateBody(current, {
				...draftOf(current),
				name: 'Nouveau',
				projectId: '',
				ifOpen: 'queue',
				timezone: 'UTC',
			}),
		).toEqual({
			name: 'Nouveau',
			project_id: null,
			if_open: 'queue',
			timezone: 'UTC',
		})
	})
})

describe('runOutcome', () => {
	it('derives a label from the run and its card', () => {
		const label = (extra: Parameters<typeof routineRun>[1]) =>
			runOutcome(routineRun('x', extra)).label
		expect(label({ status: 'created', card_column: 'done' })).toBe('terminée')
		expect(label({ status: 'created', card_column: 'todo' })).toBe(
			'tâche créée',
		)
		expect(label({ status: 'skipped' })).toBe('ignorée')
		expect(label({ status: 'queued' })).toBe('en attente')
		expect(label({ status: 'pending' })).toBe('en attente')
		expect(label({ status: 'failed' })).toBe('échec')
	})
})

describe('runDuration', () => {
	it('measures the agent run', () => {
		expect(runDuration(routineRun('x'))).toBe('—')
		expect(
			runDuration(routineRun('x', { run_started_at: '2026-11-17T06:00:00Z' })),
		).toBe('en cours')
		expect(
			runDuration(
				routineRun('x', {
					run_started_at: '2026-11-17T06:00:00Z',
					run_finished_at: '2026-11-17T06:01:12Z',
				}),
			),
		).toBe('1 min 12')
		expect(
			runDuration(
				routineRun('x', {
					run_started_at: '2026-11-17T06:00:00Z',
					run_finished_at: '2026-11-17T06:00:48Z',
				}),
			),
		).toBe('48 s')
	})
})

describe('relativeTime', () => {
	const now = Date.parse('2026-11-18T10:00:00Z')
	it('reads the future and the past', () => {
		expect(relativeTime('2026-11-18T13:00:00Z', now)).toBe('dans 3 h')
		expect(relativeTime('2026-11-18T10:20:00Z', now)).toBe('dans 20 min')
		expect(relativeTime('2026-11-20T10:00:00Z', now)).toBe('dans 2 j')
		expect(relativeTime('2026-11-18T08:00:00Z', now)).toBe('il y a 2 h')
	})
})

describe('triggersSummary', () => {
	it('lists plannings and webhooks, or manual', () => {
		expect(triggersSummary(routine('r1'))).toBe('Manuelle')
		expect(
			triggersSummary(
				routine('r1', {
					triggers: [
						trigger('t1'),
						trigger('t2', { kind: 'webhook', schedule: null }),
					],
				}),
			),
		).toBe('Lun 06:30 · Webhook')
	})
})

describe('filterRoutines', () => {
	it('matches name, slug, prompt and agent name', () => {
		const items = [
			routine('a', { name: 'Newsletter' }),
			routine('b', { name: 'Revue', agent_id: 'a-legal' }),
		]
		expect(filterRoutines(items, 'news').map((r) => r.id)).toEqual(['a'])
		expect(
			filterRoutines(items, 'veille', (id) =>
				id === 'a-legal' ? 'Veille légale' : undefined,
			).map((r) => r.id),
		).toEqual(['b'])
		expect(filterRoutines(items, ' ')).toHaveLength(2)
	})
})

describe('timezoneOptions', () => {
	it('always offers the current zone and Paris', () => {
		const zones = timezoneOptions('Pacific/Tahiti')
		expect(zones).toContain('Pacific/Tahiti')
		expect(zones).toContain('Europe/Paris')
	})
})

describe('describeRoutineError', () => {
	it('explains conflicts and refusals', () => {
		expect(describeRoutineError(new ApiRequestError(409, 'c', 'x'))).toMatch(
			/slug/,
		)
		expect(
			describeRoutineError(new ApiRequestError(422, 'c', 'unknown time zone')),
		).toMatch(/unknown time zone/)
		expect(describeRoutineError(new Error('boom'))).toMatch(/Réessaie/)
	})
})
