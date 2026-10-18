import { render, screen } from '@testing-library/react'
import type { ReactNode } from 'react'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { RunsTable } from '#/domain/runs/ui/runs-table'

function run(
	id: string,
	status: Schemas.RunStatusResponse,
): Schemas.RunResponse {
	return {
		id,
		status,
		title: `Réunion ${id}`,
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

vi.mock('@tanstack/react-router', () => ({
	Link: ({
		to,
		params,
		children,
		...rest
	}: {
		to: string
		params: { runId: string }
		children: ReactNode
	}) => (
		<a href={to.replace('$runId', params.runId)} {...rest}>
			{children}
		</a>
	),
}))

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

	it('links each title to the run detail page', () => {
		render(<RunsTable runs={[run('7', 'queued')]} />)

		expect(
			screen.getByRole('link', { name: 'Réunion 7' }).getAttribute('href'),
		).toBe('/runs/7')
	})

	it('marks a card run with its repository and a card title', () => {
		render(
			<RunsTable
				runs={[
					{
						...run('8', 'running'),
						title: 'Corriger le bug',
						transcript_id: null,
						transcript_title: null,
						card_id: 'c8',
						repository: 'acme/web',
					},
					run('9', 'queued'),
				]}
			/>,
		)

		expect(screen.getByRole('link', { name: 'Corriger le bug' })).toBeTruthy()
		expect(screen.getAllByText('Exécution')).toHaveLength(1)
		expect(screen.getByText('acme/web').className).toContain('font-mono')
	})

	it('never renders a blank title', () => {
		render(
			<RunsTable
				runs={[
					{
						...run('a1b2c3d4e5', 'queued'),
						title: null,
						transcript_title: null,
						card_id: 'c1',
						repository: 'acme/web',
					},
				]}
			/>,
		)

		expect(
			screen.getByRole('link', { name: 'Exécution sur acme/web' }),
		).toBeTruthy()
	})

	it('renders the empty state without runs', () => {
		render(<RunsTable runs={[]} />)

		expect(screen.getByText('Aucun run')).toBeTruthy()
		expect(screen.queryByRole('table')).toBeNull()
	})
})
