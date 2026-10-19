import { fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { RunTimeline, type TimelineStep } from '#/domain/runs/ui/run-timeline'

const T0 = Date.parse('2026-10-19T10:00:00Z')

function at(seconds: number): string {
	return new Date(T0 + seconds * 1000).toISOString()
}

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
		started_at: at(0),
		finished_at: at(1),
		...extra,
	}
}

const run: Schemas.RunResponse = {
	id: 'r1',
	status: 'succeeded',
	initiated_by: 'ada',
	pipeline: ['extract', 'deduplicate', 'write'],
	created_at: at(-1),
	started_at: at(0),
	finished_at: at(40),
}

const decisions: Schemas.DecisionResponse[] = [
	{
		action: 'card.create',
		actor: 'writer',
		resource_kind: 'card',
		allowed: true,
		decided_at: at(1),
	},
	{
		action: 'card.delete',
		actor: 'writer',
		resource_kind: 'card',
		allowed: false,
		reason: 'rôle insuffisant',
		decided_at: at(2),
	},
]

function renderTimeline(
	steps: TimelineStep[],
	props: Partial<Parameters<typeof RunTimeline>[0]> = {},
) {
	const onSelect = vi.fn()
	const onCopy = vi.fn()
	render(
		<RunTimeline
			run={run}
			steps={steps}
			decisions={[]}
			nowMs={T0 + 100_000}
			selectedSeq={steps[0]?.seq}
			onSelect={onSelect}
			copied={false}
			onCopy={onCopy}
			{...props}
		/>,
	)
	return { onSelect, onCopy }
}

function frise() {
	return within(screen.getByRole('list', { name: 'Frise des étapes' }))
}

describe('RunTimeline', () => {
	it('draws one block per step, placed on the run axis, in seq order', () => {
		renderTimeline([
			step(1, 'deduplicate', 'succeeded', {
				started_at: at(10),
				finished_at: at(30),
				latency_ms: 20_000,
			}),
			step(0, 'extract', 'succeeded', {
				started_at: at(0),
				finished_at: at(10),
				latency_ms: 10_000,
			}),
		])

		const frise = screen.getByRole('list', { name: 'Frise des étapes' })
		const blocks = within(frise).getAllByRole('button')
		expect(blocks.map((block) => block.getAttribute('aria-label'))).toEqual([
			'Extraction, terminé, 10,0 s',
			'Dédoublonnage, terminé, 20,0 s',
		])
		expect(blocks[0].style.left).toBe('0%')
		expect(blocks[0].style.width).toBe('25%')
		expect(blocks[1].style.left).toBe('25%')
		expect(blocks[1].style.width).toBe('50%')
	})

	it('writes the axis graduations', () => {
		renderTimeline([step(0, 'extract', 'succeeded')])

		for (const label of ['0 s', '5 s', '40 s']) {
			expect(screen.getAllByText(label).length).toBeGreaterThan(0)
		}
	})

	it('stretches a running step up to now while the run is active', () => {
		renderTimeline(
			[
				step(0, 'extract', 'running', {
					started_at: at(10),
					finished_at: null,
					latency_ms: null,
				}),
			],
			{
				run: { ...run, status: 'running', finished_at: null },
				nowMs: T0 + 40_000,
			},
		)

		const block = frise().getByRole('button', { name: /^Extraction, en cours/ })
		expect(block.style.left).toBe('25%')
		expect(block.style.width).toBe('75%')
	})

	it('marks the selected step on the frise and in the list', () => {
		renderTimeline(
			[step(0, 'extract', 'succeeded'), step(1, 'write', 'failed')],
			{ selectedSeq: 1 },
		)

		expect(
			frise()
				.getByRole('button', { name: /^Rédaction/ })
				.getAttribute('aria-pressed'),
		).toBe('true')
		expect(
			frise()
				.getByRole('button', { name: /^Extraction/ })
				.getAttribute('aria-pressed'),
		).toBe('false')
		const options = within(
			screen.getByRole('list', { name: 'Étapes' }),
		).getAllByRole('button')
		expect(options[1].getAttribute('aria-current')).toBe('true')
		expect(options[0].getAttribute('aria-current')).toBeNull()
	})

	it('selects a step from the frise and from the list', async () => {
		const { onSelect } = renderTimeline([
			step(0, 'extract', 'succeeded'),
			step(1, 'write', 'succeeded'),
		])

		await userEvent.click(frise().getByRole('button', { name: /^Rédaction/ }))
		await userEvent.click(
			within(screen.getByRole('list', { name: 'Étapes' })).getAllByRole(
				'button',
			)[0],
		)

		expect(onSelect).toHaveBeenNthCalledWith(1, 1)
		expect(onSelect).toHaveBeenNthCalledWith(2, 0)
	})

	it('moves the selection with the arrow keys on the list', () => {
		const { onSelect } = renderTimeline(
			[
				step(0, 'extract', 'succeeded'),
				step(1, 'deduplicate', 'succeeded'),
				step(2, 'write', 'succeeded'),
			],
			{ selectedSeq: 1 },
		)
		const list = screen.getByRole('list', { name: 'Étapes' })

		fireEvent.keyDown(list, { key: 'ArrowDown' })
		fireEvent.keyDown(list, { key: 'ArrowUp' })

		expect(onSelect).toHaveBeenNthCalledWith(1, 2)
		expect(onSelect).toHaveBeenNthCalledWith(2, 0)
	})

	it('shows the output of the selected step as coloured JSON, with a copy button', async () => {
		const { onCopy } = renderTimeline([
			step(0, 'extract', 'failed', {
				detail: '3 actions',
				error: 'quota dépassé',
			}),
		])

		const json = within(
			screen.getByRole('figure', { name: 'Sortie au format JSON' }),
		)
			.getByText(/detail/)
			.closest('pre') as HTMLElement
		expect(json.textContent).toBe(
			'{\n  "detail": "3 actions",\n  "error": "quota dépassé"\n}',
		)
		expect(screen.getByText('"detail"').className).toContain('text-foreground')
		expect(screen.getByText('"3 actions"').className).not.toContain('signal')
		expect(screen.getByRole('alert').textContent).toBe('quota dépassé')

		await userEvent.click(screen.getByRole('button', { name: 'Copier' }))

		expect(onCopy).toHaveBeenCalledWith(json.textContent)
	})

	it('tells when a step produced nothing', () => {
		renderTimeline([step(0, 'deduplicate', 'skipped')])

		expect(screen.getByText("Cette étape n'a rien produit.")).toBeTruthy()
	})

	it('lists the tool calls under the Outils tab', async () => {
		renderTimeline([
			step(0, 'write', 'succeeded', {
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
		])

		await userEvent.click(screen.getByRole('tab', { name: 'Outils' }))

		expect(screen.getByText('create_card').className).toContain('font-mono')
		expect(screen.getByText('delete_card')).toBeTruthy()
		expect(screen.getByText('Réussi')).toBeTruthy()
		expect(screen.getByText('Échoué')).toBeTruthy()
		expect(screen.getByText('carte créée')).toBeTruthy()
	})

	it('shows agent, model, usage and timestamps under the Métadonnées tab', async () => {
		renderTimeline([
			step(0, 'write', 'succeeded', {
				agent: 'capitainerie-agent-writer',
				latency_ms: 1500,
			}),
		])

		await userEvent.click(screen.getByRole('tab', { name: 'Métadonnées' }))

		expect(screen.getByText('writer')).toBeTruthy()
		expect(screen.queryByText(/capitainerie-agent-/)).toBeNull()
		expect(screen.getByText('claude-x')).toBeTruthy()
		expect(screen.getByText('0,0123 €')).toBeTruthy()
		expect(screen.getAllByText('1,5 s').length).toBeGreaterThan(0)
		expect(screen.getByText('Début')).toBeTruthy()
		expect(screen.getByText('Fin')).toBeTruthy()
	})

	it('shows allowed and refused decisions with the reason', () => {
		renderTimeline([], { decisions })

		expect(screen.getByText('Aucune étape.')).toBeTruthy()
		expect(screen.getByText('card.create — writer — autorisé')).toBeTruthy()
		expect(
			screen.getByText('card.delete — writer — refusé (rôle insuffisant)'),
		).toBeTruthy()
	})

	it('shows the pull request, branch and dropped tool calls of an execute step', () => {
		renderTimeline([
			step(0, 'execute', 'succeeded', {
				execution: {
					branch: 'capitainerie/0123456789ab',
					pull_request: 'https://github.com/acme/web/pull/7',
					no_changes: false,
					dropped_tool_calls: 2,
				},
			}),
		])

		expect(
			screen
				.getByRole('link', { name: 'Pull request en brouillon' })
				.getAttribute('href'),
		).toBe('https://github.com/acme/web/pull/7')
		expect(screen.getByText("2 appels d'outils non affichés")).toBeTruthy()
		expect(
			screen.getAllByText(/capitainerie\/0123456789ab/).length,
		).toBeGreaterThan(0)
	})

	it('shows no change and a failed execution reason', () => {
		renderTimeline([
			step(0, 'execute', 'succeeded', {
				execution: { no_changes: true },
			}),
		])
		expect(screen.getByText('Aucun changement')).toBeTruthy()
	})

	it('shows a failed execution reason', () => {
		renderTimeline([
			step(1, 'execute', 'failed', {
				execution: { error: 'le push a été refusé' },
			}),
		])

		expect(screen.getAllByText(/le push a été refusé/).length).toBeGreaterThan(
			0,
		)
	})
})
