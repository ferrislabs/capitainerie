import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { RunsTable } from '#/domain/runs/ui/runs-table'

function run(
	id: string,
	status: Schemas.RunStatusResponse,
): Schemas.RunResponse {
	return {
		id,
		status,
		transcript_id: `t-${id}`,
		transcript_title: `Réunion ${id}`,
		initiated_by: 'ada',
		pipeline: ['résumé', 'actions'],
		failure: null,
		created_at: '2026-10-16T09:30:00Z',
		started_at: null,
		finished_at: null,
	}
}

describe('RunsTable', () => {
	it('renders French status labels, titles and pipeline', () => {
		render(
			<RunsTable
				runs={[
					run('1', 'queued'),
					run('2', 'running'),
					run('3', 'succeeded'),
					run('4', 'failed'),
					run('5', 'cancelled'),
				]}
			/>,
		)

		for (const label of [
			'En attente',
			'En cours',
			'Terminé',
			'Échec',
			'Annulé',
		]) {
			expect(screen.getByText(label)).toBeTruthy()
		}
		expect(screen.getByText('Réunion 3')).toBeTruthy()
		expect(screen.getAllByText('résumé → actions')).toHaveLength(5)
	})

	it('renders the empty state without runs', () => {
		render(<RunsTable runs={[]} />)

		expect(screen.getByText('Aucun run')).toBeTruthy()
		expect(screen.queryByRole('table')).toBeNull()
	})
})
