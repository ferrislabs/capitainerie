import { screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { HomeCards } from '#/domain/home/ui/home-cards'

async function setup(props: Parameters<typeof HomeCards>[0]) {
	await renderAtPath(<HomeCards {...props} />, '/runs')
}

describe('HomeCards', () => {
	it('shows the four cards with their figures and links', async () => {
		await setup({ toValidate: 3, active: 2, monthCostMicros: 1_500_000 })

		const validate = screen.getByRole('link', { name: /À valider/ })
		expect(validate.getAttribute('href')).toBe('/kanban')
		expect(validate.textContent).toContain('3')
		expect(validate.textContent).toContain('cartes proposées')

		const active = screen.getByRole('link', { name: /Agents en cours/ })
		expect(active.getAttribute('href')).toBe('/live')
		expect(active.textContent).toContain('2')

		const cost = screen.getByRole('link', { name: /Coût du mois/ })
		expect(cost.getAttribute('href')).toBe('/agents')
		expect(cost.textContent).toContain('1,5000 €')

		const submit = screen.getByRole('link', { name: /Déposer une réunion/ })
		expect(submit.getAttribute('href')).toBe('/runs')
	})

	it('uses solid yellow only when a card waits for validation', async () => {
		await setup({ toValidate: 1, active: 0, monthCostMicros: 0 })

		const count = screen.getByText('1')
		expect(count.className).toContain('bg-signal-attention')
		expect(screen.getByText('carte proposée')).toBeTruthy()
	})

	it('keeps the validation card quiet at zero', async () => {
		await setup({ toValidate: 0, active: 0, monthCostMicros: 0 })

		const validate = screen.getByRole('link', { name: /À valider/ })
		expect(validate.innerHTML).not.toContain('bg-signal-attention')
		expect(validate.textContent).toContain('Rien à valider')
	})

	it('marks unavailable figures with a word', async () => {
		await setup({ toValidate: null, active: null, monthCostMicros: null })

		expect(screen.getAllByText('Indisponible')).toHaveLength(3)
	})

	it('hides figures while loading', async () => {
		await setup({
			toValidate: undefined,
			active: undefined,
			monthCostMicros: undefined,
		})

		expect(screen.getAllByText('Chargement')).toHaveLength(3)
	})
})
