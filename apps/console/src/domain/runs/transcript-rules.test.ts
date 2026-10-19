import { describe, expect, it } from 'vitest'
import {
	addParticipants,
	PARTICIPANT_MAX_LENGTH,
	PARTICIPANTS_MAX,
	validateParticipants,
	validateRepository,
} from '#/domain/runs/transcript-rules'

describe('addParticipants', () => {
	it('splits on commas, trims and skips blanks and duplicates', () => {
		expect(addParticipants(['Alice'], ' Bob, , Alice,Claire ')).toEqual([
			'Alice',
			'Bob',
			'Claire',
		])
	})

	it('keeps the list when nothing is typed', () => {
		expect(addParticipants(['Alice'], '  ')).toEqual(['Alice'])
	})
})

describe('validateParticipants', () => {
	it('accepts the limits', () => {
		const names = Array.from({ length: PARTICIPANTS_MAX }, (_, n) => `P${n}`)
		expect(validateParticipants(names)).toBe(undefined)
		expect(validateParticipants(['x'.repeat(PARTICIPANT_MAX_LENGTH)])).toBe(
			undefined,
		)
	})

	it('refuses too many or too long names', () => {
		const names = Array.from(
			{ length: PARTICIPANTS_MAX + 1 },
			(_, n) => `P${n}`,
		)
		expect(validateParticipants(names)).toMatch(/50 participants/)
		expect(
			validateParticipants(['x'.repeat(PARTICIPANT_MAX_LENGTH + 1)]),
		).toMatch(/100 caractères/)
	})
})

describe('validateRepository', () => {
	it('accepts empty and owner/name', () => {
		expect(validateRepository('')).toBe(undefined)
		expect(validateRepository(' acme/web.app ')).toBe(undefined)
	})

	it('refuses anything else', () => {
		for (const value of ['acme', 'acme/', '/web', 'a/b/c', 'a b/c']) {
			expect(validateRepository(value)).toMatch(/propriétaire\/nom/)
		}
	})
})
