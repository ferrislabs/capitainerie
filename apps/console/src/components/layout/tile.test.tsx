import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { Tile } from '#/components/layout/tile'

describe('Tile', () => {
	it('shows label, value and hint', () => {
		render(<Tile label="Réussite" value="9 / 10" hint="derniers passages" />)
		expect(screen.getByText('Réussite')).toBeTruthy()
		expect(screen.getByText('9 / 10')).toBeTruthy()
		expect(screen.getByText('derniers passages')).toBeTruthy()
	})
})
