import { describe, expect, it } from 'vitest'
import {
	hasActiveWorksite,
	worksiteStatus,
} from '#/domain/worksites/worksite-status'

describe('worksiteStatus', () => {
	it.each([
		['queued', 'in_progress', 'En file', 'neutral'],
		['running', 'in_progress', 'En cours', 'running'],
		['succeeded', 'in_progress', 'À relire', 'attention'],
		['failed', 'in_progress', 'Échec', 'danger'],
		['cancelled', 'todo', 'Annulé', 'neutral'],
	])('reads %s on the latest run', (run_status, column, label, tone) => {
		expect(worksiteStatus({ run_status, column })).toEqual({ label, tone })
	})

	it('falls back on the column when the card never ran', () => {
		expect(worksiteStatus({ run_status: null, column: 'done' })).toEqual({
			label: 'Terminé',
			tone: 'success',
		})
		expect(worksiteStatus({ column: 'todo' })).toEqual({
			label: 'À lancer',
			tone: 'neutral',
		})
	})
})

describe('hasActiveWorksite', () => {
	it('is true while a run is queued or running', () => {
		expect(hasActiveWorksite([{ run_status: 'succeeded' }])).toBe(false)
		expect(
			hasActiveWorksite([
				{ run_status: 'succeeded' },
				{ run_status: 'queued' },
			]),
		).toBe(true)
		expect(hasActiveWorksite([{ run_status: 'running' }])).toBe(true)
		expect(hasActiveWorksite(undefined)).toBe(false)
	})
})
