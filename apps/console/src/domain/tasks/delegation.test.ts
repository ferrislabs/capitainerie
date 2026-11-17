import { describe, expect, it } from 'vitest'
import { canDelegate } from '#/domain/tasks/delegation'

const limits = { maxDepth: 2, maxParallel: 3 }

describe('canDelegate', () => {
	it('refuses an empty title', () => {
		expect(
			canDelegate({ title: '  ', depth: 0, openChildren: 0, ...limits }),
		).toEqual({ ok: false, reason: 'Écris le titre de la sous-tâche' })
	})

	it('refuses when depth would exceed the limit', () => {
		expect(
			canDelegate({ title: 'x', depth: 2, openChildren: 0, ...limits }),
		).toEqual({ ok: false, reason: 'Profondeur maximale atteinte (2)' })
	})

	it('refuses when parallel limit is reached', () => {
		expect(
			canDelegate({ title: 'x', depth: 0, openChildren: 3, ...limits }),
		).toEqual({
			ok: false,
			reason: 'Limite atteinte : 3 sous-tâches en parallèle',
		})
	})

	it('accepts within limits', () => {
		expect(
			canDelegate({ title: 'x', depth: 1, openChildren: 2, ...limits }),
		).toEqual({ ok: true })
	})
})
