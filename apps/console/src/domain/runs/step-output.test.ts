import { describe, expect, it } from 'vitest'
import { stepOutputValue } from '#/domain/runs/step-output'

describe('stepOutputValue', () => {
	it('has nothing to show for a step without detail, execution or error', () => {
		expect(stepOutputValue({})).toBeUndefined()
		expect(stepOutputValue({ detail: '', error: null })).toBeUndefined()
	})

	it('gathers the detail, the execution and the error under their own keys', () => {
		expect(
			stepOutputValue({
				detail:
					'2 action(s) anchored in the transcript, 1 dropped (quote not found)',
				execution: { branch: 'capitainerie/x', no_changes: false },
				error: 'quota dépassé',
			}),
		).toEqual({
			detail:
				'2 action(s) ancrée(s) dans le transcript, 1 écartée(s) (citation introuvable)',
			execution: { branch: 'capitainerie/x', no_changes: false },
			error: 'quota dépassé',
		})
	})
})
