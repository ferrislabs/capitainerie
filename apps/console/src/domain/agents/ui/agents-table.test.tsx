import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { buildRows } from '#/domain/agents/filters'
import { AgentsTable } from '#/domain/agents/ui/agents-table'
import type { ModelAssignments } from '#/domain/settings/roles'

function agent(id: string): Schemas.AgentResponse {
	return {
		agent: id,
		runs: 3,
		last_seen: '2026-10-16T20:05:19Z',
		input_tokens: 2735,
		output_tokens: 309,
		cost_micros: 1_500_000,
	}
}

const ASSIGNMENTS: ModelAssignments = {
	extractor: { provider: 'anthropic', model: 'claude-sonnet' },
	judge: { provider: 'anthropic', model: 'claude-sonnet' },
	writer: { provider: 'anthropic', model: 'claude-haiku' },
	embeddings: { provider: 'mistral', model: 'mistral-embed' },
	executor: 'claude-opus',
}

const rows = buildRows(
	[agent('capitainerie-agent-writer'), agent('capitainerie-agent-mystery')],
	ASSIGNMENTS,
)

describe('AgentsTable', () => {
	it('renders the French headers', () => {
		render(<AgentsTable rows={rows} onOpen={vi.fn()} />)

		for (const header of [
			'Agent',
			'Description',
			'Source',
			'Modèle',
			'Runs',
			'Dernière activité',
			'Coût',
			'Identifiant',
		]) {
			expect(screen.getByRole('columnheader', { name: header })).toBeTruthy()
		}
	})

	it('shows catalog data for known agents and the id for unknown ones', () => {
		render(<AgentsTable rows={rows} onOpen={vi.fn()} />)

		const known = screen.getAllByRole('row')[1] as HTMLElement
		expect(within(known).getByText('Rédacteur')).toBeTruthy()
		expect(within(known).getByText('Pipeline')).toBeTruthy()
		expect(within(known).getByText('claude-haiku')).toBeTruthy()
		expect(
			within(known).getByText('capitainerie-agent-writer').className,
		).toContain('font-mono')

		const unknown = screen.getAllByRole('row')[2] as HTMLElement
		expect(
			within(unknown).getByRole('button', {
				name: 'capitainerie-agent-mystery',
			}),
		).toBeTruthy()
	})

	it('formats the cost like the runs page', () => {
		render(<AgentsTable rows={rows} onOpen={vi.fn()} />)

		expect(screen.getAllByText(/1,5000/)).toHaveLength(2)
	})

	it('opens the panel from the row and from the name', async () => {
		const onOpen = vi.fn()
		render(<AgentsTable rows={rows} onOpen={onOpen} />)

		await userEvent.click(screen.getByRole('button', { name: 'Rédacteur' }))
		expect(onOpen).toHaveBeenCalledTimes(1)
		expect(onOpen).toHaveBeenLastCalledWith(rows[0])

		await userEvent.click(screen.getAllByText('3')[1] as HTMLElement)
		expect(onOpen).toHaveBeenCalledTimes(2)
		expect(onOpen).toHaveBeenLastCalledWith(rows[1])
	})

	it('copies the id without opening the panel', async () => {
		const writeText = vi.fn().mockResolvedValue(undefined)
		Object.defineProperty(navigator, 'clipboard', {
			value: { writeText },
			configurable: true,
		})
		const onOpen = vi.fn()
		render(<AgentsTable rows={rows} onOpen={onOpen} />)

		await userEvent.click(
			screen.getAllByRole('button', {
				name: 'Copier l’identifiant',
			})[0] as HTMLElement,
		)

		expect(writeText).toHaveBeenCalledWith('capitainerie-agent-writer')
		expect(onOpen).not.toHaveBeenCalled()
	})
})
