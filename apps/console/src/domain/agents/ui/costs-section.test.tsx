import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { CostsSection } from '#/domain/agents/ui/costs-section'

const costs: Schemas.CostsResponse = {
	rows: [
		{
			key: 'extractor',
			runs: 4,
			input_tokens: 2510,
			output_tokens: 983,
			cost_micros: 2_000_000,
			latency_ms_p50: 58_400,
		},
		{
			key: 'unknown',
			runs: 3,
			input_tokens: 0,
			output_tokens: 0,
			cost_micros: 0,
			latency_ms_p50: null,
		},
	],
	total: { input_tokens: 2510, output_tokens: 983, cost_micros: 2_000_000 },
}

function renderSection(
	overrides: Partial<Parameters<typeof CostsSection>[0]> = {},
) {
	const onChange = vi.fn()
	render(
		<CostsSection
			by="agent"
			costs={costs}
			isPending={false}
			isError={false}
			onChange={onChange}
			{...overrides}
		/>,
	)
	return onChange
}

describe('CostsSection', () => {
	it('offers the four groupings and marks the active one', () => {
		renderSection({ by: 'user' })

		for (const label of [
			'Par agent',
			'Par utilisateur',
			'Par modèle',
			'Par jour',
		]) {
			expect(screen.getByRole('button', { name: label })).toBeTruthy()
		}
		expect(
			screen
				.getByRole('button', { name: 'Par utilisateur' })
				.getAttribute('aria-pressed'),
		).toBe('true')
		expect(
			screen.getByRole('columnheader', { name: 'Utilisateur' }),
		).toBeTruthy()
	})

	it('reports the chosen grouping', async () => {
		const onChange = renderSection()

		await userEvent.click(screen.getByRole('button', { name: 'Par modèle' }))

		expect(onChange).toHaveBeenCalledWith('model')
	})

	it('renders rows, formatted latency and the total', () => {
		renderSection()

		expect(screen.getByText('extractor')).toBeTruthy()
		expect(screen.getByText('58,4 s')).toBeTruthy()
		expect(screen.getByText('—')).toBeTruthy()
		expect(screen.getAllByText(/2,0000/)).toHaveLength(2)
		expect(screen.getByText('Total')).toBeTruthy()
	})

	it('renders the loading, error and empty states', () => {
		const { unmount } = render(
			<CostsSection
				by="agent"
				costs={undefined}
				isPending
				isError={false}
				onChange={vi.fn()}
			/>,
		)
		expect(screen.getByText('Chargement…')).toBeTruthy()
		unmount()

		const failed = render(
			<CostsSection
				by="agent"
				costs={undefined}
				isPending={false}
				isError
				onChange={vi.fn()}
			/>,
		)
		expect(screen.getByRole('alert')).toBeTruthy()
		failed.unmount()

		render(
			<CostsSection
				by="agent"
				costs={{ rows: [], total: costs.total }}
				isPending={false}
				isError={false}
				onChange={vi.fn()}
			/>,
		)
		expect(screen.getByText('Aucun coût')).toBeTruthy()
	})
})
