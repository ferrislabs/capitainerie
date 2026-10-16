import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { AgentsTable } from '#/domain/agents/ui/agents-table'

function agent(name: string): Schemas.AgentResponse {
	return {
		agent: name,
		runs: 3,
		last_seen: '2026-10-16T20:05:19Z',
		input_tokens: 2735,
		output_tokens: 309,
		cost_micros: 1_500_000,
	}
}

describe('AgentsTable', () => {
	it('renders the French headers and one row per agent', () => {
		render(<AgentsTable agents={[agent('extractor'), agent('writer')]} />)

		for (const header of [
			'Agent',
			'Runs',
			'Dernière activité',
			'Tokens entrée/sortie',
			'Coût',
		]) {
			expect(screen.getByRole('columnheader', { name: header })).toBeTruthy()
		}
		expect(screen.getByText('extractor')).toBeTruthy()
		expect(screen.getByText('writer')).toBeTruthy()
		expect(screen.getAllByText(/309/)).toHaveLength(2)
	})

	it('formats the cost like the runs page', () => {
		render(<AgentsTable agents={[agent('extractor')]} />)

		expect(screen.getByText(/1,5000/)).toBeTruthy()
	})

	it('renders the empty state without agents', () => {
		render(<AgentsTable agents={[]} />)

		expect(screen.getByText('Aucun agent')).toBeTruthy()
		expect(screen.queryByRole('table')).toBeNull()
	})
})
