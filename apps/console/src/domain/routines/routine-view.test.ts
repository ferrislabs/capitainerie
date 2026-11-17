import { describe, expect, it } from 'vitest'
import { buildRoutines } from '#/domain/routines/mock/routines'
import {
	agentOptions,
	planningOf,
	scheduleLabel,
	upcomingRuns,
} from '#/domain/routines/routine-view'

const NOW = new Date(2026, 10, 17, 10, 0)
const routines = buildRoutines()
const board = routines[0]
const paused = routines[2]
if (!board || !paused) throw new Error('fixtures')

describe('routine view', () => {
	it('reads the planning trigger and its label', () => {
		expect(planningOf(board)?.kind).toBe('planning')
		expect(scheduleLabel(board)).toBe('Lun. 06:30')
		expect(scheduleLabel({ ...board, triggers: [] })).toBe('Manuelle')
	})
	it('computes upcoming runs only for an active routine', () => {
		expect(upcomingRuns(board, NOW, 3)).toHaveLength(3)
		expect(upcomingRuns({ ...board, enabled: false }, NOW, 3)).toEqual([])
		expect(upcomingRuns(paused, NOW, 3)).toEqual([])
	})
	it('lists unique agent names, demo agents first', () => {
		const names = agentOptions(
			['Zed', 'Board Analyst'],
			['Zed', 'Alpha', 'Zed'],
		)
		expect(names).toEqual(['Zed', 'Board Analyst', 'Alpha'])
	})
})
