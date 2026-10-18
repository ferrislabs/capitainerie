import { describe, expect, it } from 'vitest'
import { stepDetailText, toolSummaryText } from '#/domain/runs/step-text'

describe('step texts', () => {
	it('translates the extraction summary', () => {
		expect(
			stepDetailText(
				'3 action(s) anchored in the transcript, 0 dropped (quote not found)',
			),
		).toBe(
			'3 action(s) ancrée(s) dans le transcript, 0 écartée(s) (citation introuvable)',
		)
	})

	it('translates the comment tool summary', () => {
		expect(toolSummaryText('comment on card 01a1')).toBe(
			'commentaire sur la carte 01a1',
		)
	})

	it('passes unknown texts through', () => {
		expect(stepDetailText('3 actions')).toBe('3 actions')
		expect(toolSummaryText('carte créée')).toBe('carte créée')
	})
})
