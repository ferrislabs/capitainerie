import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Tile } from '#/components/layout/tile'

describe('DemoBanner', () => {
	it('says the data is for demonstration', () => {
		render(<DemoBanner />)
		expect(screen.getByRole('status').textContent).toContain(
			'Données de démonstration',
		)
	})
})

describe('Tile', () => {
	it('shows label, value and hint', () => {
		render(<Tile label="Réussite" value="9 / 10" hint="derniers passages" />)
		expect(screen.getByText('Réussite')).toBeTruthy()
		expect(screen.getByText('9 / 10')).toBeTruthy()
		expect(screen.getByText('derniers passages')).toBeTruthy()
	})
})
