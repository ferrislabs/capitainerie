import { describe, expect, it } from 'vitest'
import {
	assigneeName,
	criteriaProgress,
	formatDueDate,
	initials,
	isFlagged,
	isOverdue,
	splitLabels,
} from '#/domain/kanban/card-details'

describe('card details', () => {
	it('flags urgent and high priorities only', () => {
		expect(isFlagged('urgent')).toBe(true)
		expect(isFlagged('high')).toBe(true)
		expect(isFlagged('normal')).toBe(false)
		expect(isFlagged('low')).toBe(false)
	})

	it('is overdue only once the due day has passed', () => {
		const now = new Date(2026, 9, 19, 23, 59)
		expect(isOverdue('2026-10-18', now)).toBe(true)
		expect(isOverdue('2026-10-19', now)).toBe(false)
		expect(isOverdue('2026-10-20', now)).toBe(false)
		expect(isOverdue(null, now)).toBe(false)
		expect(isOverdue(undefined, now)).toBe(false)
	})

	it('formats a due date in French without a timezone shift', () => {
		expect(formatDueDate('2026-10-31')).toContain('31')
		expect(formatDueDate('2026-10-31')).toContain('2026')
		expect(formatDueDate('n/a')).toBe('n/a')
	})

	it('takes the initials of the first and last words', () => {
		expect(initials('Claire Martin')).toBe('CM')
		expect(initials('claire')).toBe('CL')
		expect(initials('Jean-Pierre de la Tour')).toBe('JT')
		expect(initials('  ')).toBe('?')
		expect(initials('élodie durand')).toBe('ÉD')
	})

	it('counts the criteria done, and nothing without criteria', () => {
		expect(criteriaProgress([])).toBeNull()
		expect(
			criteriaProgress([
				{ text: 'a', done: true },
				{ text: 'b', done: false },
				{ text: 'c', done: true },
			]),
		).toEqual({ done: 2, total: 3 })
	})

	it('shows two labels then a count of the others', () => {
		expect(splitLabels(['a', 'b'])).toEqual({ shown: ['a', 'b'], hidden: 0 })
		expect(splitLabels(['a', 'b', 'c', 'd'])).toEqual({
			shown: ['a', 'b'],
			hidden: 2,
		})
		expect(splitLabels([])).toEqual({ shown: [], hidden: 0 })
	})

	it('names an assignee by member, else by subject', () => {
		const names = new Map([['u1', 'Claire Martin']])
		expect(assigneeName('u1', names)).toBe('Claire Martin')
		expect(assigneeName('u2', names)).toBe('u2')
		expect(assigneeName(null, names)).toBeUndefined()
	})
})
