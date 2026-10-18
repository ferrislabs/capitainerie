import { render, screen, within } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { RunTimeline, type TimelineStep } from '#/domain/runs/ui/run-timeline'

function step(
	seq: number,
	kind: string,
	status: Schemas.StepStatusResponse,
	extra: Partial<TimelineStep> = {},
): TimelineStep {
	return {
		seq,
		kind,
		status,
		agent: null,
		model: 'claude-x',
		input_tokens: 1200,
		output_tokens: 300,
		cost_micros: 12_345,
		latency_ms: 850,
		tool_calls: [],
		started_at: '2026-10-16T09:30:00Z',
		...extra,
	}
}

const decisions: Schemas.DecisionResponse[] = [
	{
		action: 'card.create',
		actor: 'writer',
		resource_kind: 'card',
		allowed: true,
		decided_at: '2026-10-16T09:30:01Z',
	},
	{
		action: 'card.delete',
		actor: 'writer',
		resource_kind: 'card',
		allowed: false,
		reason: 'rôle insuffisant',
		decided_at: '2026-10-16T09:30:02Z',
	},
]

describe('RunTimeline', () => {
	it('renders steps in seq order with French labels and details', () => {
		render(
			<RunTimeline
				steps={[
					step(2, 'write', 'failed', {
						agent: 'capitainerie-agent-writer',
						error: 'quota dépassé',
						latency_ms: 1500,
						tool_calls: [
							{
								tool: 'create_card',
								succeeded: true,
								latency_ms: 40,
								summary: 'carte créée',
							},
							{
								tool: 'delete_card',
								succeeded: false,
								latency_ms: 12,
								summary: 'refusé',
							},
						],
					}),
					step(0, 'extract', 'succeeded', { detail: '3 actions' }),
					step(1, 'deduplicate', 'running'),
					step(3, 'other', 'skipped'),
				]}
				decisions={[]}
			/>,
		)

		const items = screen.getAllByRole('listitem')
		expect(within(items[0]).getByText('Extraction')).toBeTruthy()
		expect(screen.getByText('Dédoublonnage')).toBeTruthy()
		expect(screen.getByText('Rédaction')).toBeTruthy()
		for (const label of ['Terminé', 'En cours', 'Échec', 'Ignoré']) {
			expect(screen.getByText(label)).toBeTruthy()
		}
		expect(screen.getByText('3 actions')).toBeTruthy()
		expect(screen.getByText('quota dépassé')).toBeTruthy()
		expect(screen.getAllByText(/writer/).length).toBeGreaterThan(0)
		expect(screen.queryByText(/capitainerie-agent-/)).toBeNull()
		expect(screen.getAllByText(/0,0123 €/).length).toBe(4)
		expect(screen.getByText(/1,5 s/)).toBeTruthy()
		expect(screen.getByText('create_card')).toBeTruthy()
		expect(screen.getByText('delete_card')).toBeTruthy()
		expect(screen.getByLabelText('Réussi')).toBeTruthy()
		expect(screen.getByLabelText('Échoué')).toBeTruthy()
		expect(screen.getByText('carte créée')).toBeTruthy()
	})

	it('shows allowed and refused decisions with the reason', () => {
		render(<RunTimeline steps={[]} decisions={decisions} />)

		expect(screen.getByText('card.create — writer — autorisé')).toBeTruthy()
		expect(
			screen.getByText('card.delete — writer — refusé (rôle insuffisant)'),
		).toBeTruthy()
	})

	it('renders an execute step with its tool calls, pull request and branch', () => {
		render(
			<RunTimeline
				steps={[
					step(0, 'execute', 'succeeded', {
						agent: 'capitainerie-agent-executor',
						model: 'claude-sonnet-5',
						tool_calls: [
							{
								tool: 'Edit',
								succeeded: true,
								latency_ms: 30,
								summary: 'src/main.rs',
							},
						],
						execution: {
							branch: 'capitainerie/0123456789ab',
							pull_request: 'https://github.com/acme/web/pull/7',
							no_changes: false,
							dropped_tool_calls: 2,
						},
					}),
				]}
				decisions={[]}
			/>,
		)

		expect(screen.getByText('Exécution')).toBeTruthy()
		expect(screen.getByText(/executor/)).toBeTruthy()
		expect(screen.getByText(/claude-sonnet-5/)).toBeTruthy()
		expect(screen.getByText('Edit').className).toContain('font-mono')
		expect(screen.getByText('src/main.rs')).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'Pull request en brouillon' })
				.getAttribute('href'),
		).toBe('https://github.com/acme/web/pull/7')
		expect(screen.getByText('capitainerie/0123456789ab')).toBeTruthy()
		expect(screen.getByText("2 appels d'outils non affichés")).toBeTruthy()
	})

	it('shows no change and a failed execution reason', () => {
		render(
			<RunTimeline
				steps={[
					step(0, 'execute', 'succeeded', {
						execution: { no_changes: true },
					}),
					step(1, 'execute', 'failed', {
						execution: { error: 'le push a été refusé' },
					}),
				]}
				decisions={[]}
			/>,
		)

		expect(screen.getByText('Aucun changement')).toBeTruthy()
		expect(screen.getByText('le push a été refusé')).toBeTruthy()
	})
})
