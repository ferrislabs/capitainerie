import { describe, expect, it } from 'vitest'
import {
	addCriterion,
	addLabel,
	MAX_CRITERIA,
	MAX_LABELS,
	removeCriterion,
	removeLabel,
	renameCriterion,
	toggleCriterion,
} from '#/domain/kanban/card-patch'

const CRITERIA = [
	{ text: 'a', done: false },
	{ text: 'b', done: true },
]

describe('card patch helpers', () => {
	it('toggles, renames and removes one criterion without touching the others', () => {
		expect(toggleCriterion(CRITERIA, 0)).toEqual([
			{ text: 'a', done: true },
			{ text: 'b', done: true },
		])
		expect(renameCriterion(CRITERIA, 1, '  c ')).toEqual([
			{ text: 'a', done: false },
			{ text: 'c', done: true },
		])
		expect(removeCriterion(CRITERIA, 0)).toEqual([{ text: 'b', done: true }])
		expect(CRITERIA[0]?.done).toBe(false)
	})

	it('adds a trimmed criterion, not a blank one, not past the limit', () => {
		expect(addCriterion(CRITERIA, ' d ')?.at(-1)).toEqual({
			text: 'd',
			done: false,
		})
		expect(addCriterion(CRITERIA, '   ')).toBeUndefined()
		const full = Array.from({ length: MAX_CRITERIA }, (_, n) => ({
			text: `c${n}`,
			done: false,
		}))
		expect(addCriterion(full, 'x')).toBeUndefined()
	})

	it('adds a label once without case, and removes it', () => {
		expect(addLabel(['Infra'], ' ci ')).toEqual(['Infra', 'ci'])
		expect(addLabel(['Infra'], 'infra')).toBeUndefined()
		expect(addLabel([], ' ')).toBeUndefined()
		expect(
			addLabel(
				Array.from({ length: MAX_LABELS }, (_, n) => `l${n}`),
				'x',
			),
		).toBeUndefined()
		expect(removeLabel(['a', 'b'], 'a')).toEqual(['b'])
	})
})
