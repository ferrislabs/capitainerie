import { screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { buildRows } from '#/domain/agents/filters'
import { AgentSheet } from '#/domain/agents/ui/agent-sheet'

const [row] = buildRows(
	[
		{
			agent: 'capitainerie-agent-executor',
			runs: 4,
			last_seen: '2026-10-16T20:05:19Z',
			input_tokens: 2735,
			output_tokens: 309,
			cost_micros: 1_500_000,
		},
	],
	{
		extractor: { provider: 'anthropic', model: 'a' },
		judge: { provider: 'anthropic', model: 'a' },
		writer: { provider: 'anthropic', model: 'a' },
		embeddings: { provider: 'mistral', model: 'e' },
		executor: 'claude-opus',
	},
)

const [captain] = buildRows(
	[
		{
			agent: 'capitainerie-agent-captain',
			runs: 2,
			last_seen: '2026-10-16T20:05:19Z',
			input_tokens: 10,
			output_tokens: 5,
			cost_micros: 100,
		},
	],
	{
		extractor: { provider: 'anthropic', model: 'a' },
		judge: { provider: 'anthropic', model: 'a' },
		writer: { provider: 'anthropic', model: 'claude-haiku' },
		embeddings: { provider: 'mistral', model: 'e' },
		captain: null,
	},
)

describe('AgentSheet', () => {
	it('names the Captain and shows the writer model it falls back to', async () => {
		await renderAtPath(
			<AgentSheet row={captain} onClose={vi.fn()} />,
			'/agents',
		)

		expect(screen.getByText('Capitaine')).toBeTruthy()
		expect(screen.getByText('claude-haiku (Rédacteur)')).toBeTruthy()
	})

	it('shows role, model and stats and says the runs list is missing', async () => {
		await renderAtPath(<AgentSheet row={row} onClose={vi.fn()} />, '/agents')

		expect(screen.getByRole('dialog')).toBeTruthy()
		expect(screen.getByText('Rôle')).toBeTruthy()
		expect(screen.getAllByText('Exécutant').length).toBeGreaterThan(0)
		expect(screen.getByText('claude-opus')).toBeTruthy()
		expect(screen.getByText('Statistiques')).toBeTruthy()
		expect(screen.getByText('4')).toBeTruthy()
		expect(screen.getByText(/ne filtre pas les runs par agent/)).toBeTruthy()
	})

	it('renders nothing visible while closed', async () => {
		await renderAtPath(
			<AgentSheet row={undefined} onClose={vi.fn()} />,
			'/agents',
		)

		expect(screen.queryByRole('dialog')).toBeNull()
	})
})
