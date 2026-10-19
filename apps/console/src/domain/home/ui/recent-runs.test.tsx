import { screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { renderAtPath } from '#/components/layout/test-router'
import { RECENT_RUNS_COUNT, RecentRuns } from '#/domain/home/ui/recent-runs'

function run(index: number): Schemas.RunResponse {
	return {
		id: `0000000${index}-aaaa-bbbb-cccc-dddddddddddd`,
		status: index % 2 === 0 ? 'succeeded' : 'running',
		pipeline: [],
		initiated_by: 'ada',
		created_at: '2026-10-16T20:05:19Z',
		title: `Réunion ${index}`,
	}
}

describe('RecentRuns', () => {
	it('lists the five latest runs with their status', async () => {
		const runs = Array.from({ length: 8 }, (_, index) => run(index))
		await renderAtPath(<RecentRuns runs={runs} />, '/runs')

		expect(RECENT_RUNS_COUNT).toBe(5)
		expect(screen.getAllByRole('listitem')).toHaveLength(5)
		expect(screen.getByText('Réunion 0')).toBeTruthy()
		expect(screen.queryByText('Réunion 5')).toBeNull()
		expect(screen.getAllByText('Terminé').length).toBeGreaterThan(0)
		expect(screen.getAllByText('En cours').length).toBeGreaterThan(0)
	})

	it('shows an empty card with an action', async () => {
		await renderAtPath(<RecentRuns runs={[]} />, '/runs')

		expect(screen.getByText('Aucune activité')).toBeTruthy()
		const action = screen.getByRole('link', { name: 'Déposer une réunion' })
		expect(action.getAttribute('href')).toBe('/runs')
	})
})
