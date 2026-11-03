import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { isCardRun, isInterventionRun, runTitle } from '#/domain/runs/run-title'

function run(overrides: Partial<Schemas.RunResponse>): Schemas.RunResponse {
	return {
		id: 'abcdef012345',
		status: 'queued',
		initiated_by: 'ada',
		pipeline: [],
		created_at: '2026-10-16T09:30:00Z',
		...overrides,
	}
}

describe('runTitle', () => {
	it('prefers the title, then the transcript title', () => {
		expect(
			runTitle(run({ title: 'Corriger le bug', transcript_title: 'x' })),
		).toBe('Corriger le bug')
		expect(runTitle(run({ title: '  ', transcript_title: 'Réunion' }))).toBe(
			'Réunion',
		)
	})

	it('never returns a blank title', () => {
		expect(runTitle(run({ repository: 'acme/web' }))).toBe(
			'Exécution sur acme/web',
		)
		expect(runTitle(run({ title: null }))).toBe('Run abcdef01')
	})

	it('titles an intervention after its client', () => {
		expect(
			runTitle(
				run({
					pipeline: ['intervene'],
					card_id: 'c1',
					client: { id: 'cl1', name: 'Acme' },
				}),
			),
		).toBe('Intervention pour Acme')
		expect(runTitle(run({ pipeline: ['intervene'], card_id: 'c1' }))).toBe(
			'Intervention',
		)
		expect(
			runTitle(run({ pipeline: ['intervene'], title: 'Corriger le bug' })),
		).toBe('Corriger le bug')
	})

	it('recognises an intervention by its pipeline', () => {
		expect(isInterventionRun(run({ pipeline: ['intervene'] }))).toBe(true)
		expect(isInterventionRun(run({ pipeline: ['execute'] }))).toBe(false)
		expect(isInterventionRun(run({}))).toBe(false)
	})

	it('recognises a card run', () => {
		expect(isCardRun(run({ card_id: 'c1' }))).toBe(true)
		expect(isCardRun(run({ card_id: null }))).toBe(false)
		expect(isCardRun(run({}))).toBe(false)
	})
})
