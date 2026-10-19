import { render, screen, within } from '@testing-library/react'
import type { ReactNode } from 'react'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { ActiveRunsTable } from '#/domain/runs/ui/active-runs-table'

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

const NOW = Date.parse('2026-10-19T10:01:05Z')

function active(
	id: string,
	status: 'queued' | 'running',
	extra: Partial<Schemas.ActiveRunResponse> = {},
): Schemas.ActiveRunResponse {
	return {
		run: {
			id,
			status,
			title: `Réunion ${id}`,
			initiated_by: 'ada',
			pipeline: ['extract', 'deduplicate', 'write'],
			created_at: '2026-10-19T10:00:00Z',
			started_at: status === 'running' ? '2026-10-19T10:00:00Z' : null,
		},
		current_step: null,
		steps_done: 0,
		steps_total: 3,
		input_tokens: 0,
		output_tokens: 0,
		cost_micros: 0,
		...extra,
	}
}

describe('ActiveRunsTable', () => {
	it('says nothing runs when the list is empty', () => {
		render(<ActiveRunsTable runs={[]} nowMs={NOW} />)

		expect(screen.getByText('Aucun agent ne tourne.')).toBeTruthy()
	})

	it('shows the step, the agent, the elapsed time and the usage of a running run', () => {
		render(
			<ActiveRunsTable
				nowMs={NOW}
				runs={[
					active('r1', 'running', {
						current_step: {
							kind: 'deduplicate',
							agent: 'capitainerie-agent-writer',
							started_at: '2026-10-19T10:00:30Z',
						},
						steps_done: 1,
						input_tokens: 1200,
						output_tokens: 300,
						cost_micros: 12_345,
					}),
				]}
			/>,
		)

		const row = screen.getByRole('row', { name: /Réunion r1/ })
		expect(within(row).getByText('2/3 Dédoublonnage')).toBeTruthy()
		expect(within(row).getByText('writer')).toBeTruthy()
		expect(within(row).getByText('1 min 05 s')).toBeTruthy()
		expect(within(row).getByText('0,0123 €')).toBeTruthy()
		expect(within(row).getByRole('img', { name: 'En cours' })).toBeTruthy()
		expect(
			within(row)
				.getByRole('link', { name: 'Réunion r1' })
				.getAttribute('href'),
		).toBe('/runs/r1')
	})

	it('marks a queued run as waiting, with no agent', () => {
		render(<ActiveRunsTable nowMs={NOW} runs={[active('r2', 'queued')]} />)

		const row = screen.getByRole('row', { name: /Réunion r2/ })
		expect(within(row).getByText('En file')).toBeTruthy()
		expect(within(row).getByRole('img', { name: 'En attente' })).toBeTruthy()
		expect(within(row).getByText('—')).toBeTruthy()
	})
})
