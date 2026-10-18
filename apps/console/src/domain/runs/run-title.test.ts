import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { isCardRun, runTitle } from '#/domain/runs/run-title'

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

	it('recognises a card run', () => {
		expect(isCardRun(run({ card_id: 'c1' }))).toBe(true)
		expect(isCardRun(run({ card_id: null }))).toBe(false)
		expect(isCardRun(run({}))).toBe(false)
	})
})
