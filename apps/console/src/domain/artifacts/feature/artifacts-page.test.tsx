import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ArtifactsPage } from '#/domain/artifacts/feature/artifacts-page'

afterEach(() => vi.restoreAllMocks())

describe('ArtifactsPage', () => {
	it('renders title, banner and the cards without fetching', () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		render(<ArtifactsPage />)
		expect(screen.getByRole('heading', { name: 'Artifacts' })).toBeTruthy()
		expect(screen.getByText(/Données de démonstration/)).toBeTruthy()
		expect(screen.getAllByRole('article')).toHaveLength(6)
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('confirms the filter and the opening with a notice', async () => {
		render(<ArtifactsPage />)
		await userEvent.click(screen.getByRole('button', { name: 'Filtrer' }))
		expect(screen.getByText('Filtre par type et par agent')).toBeTruthy()
		await userEvent.click(
			screen.getByRole('button', { name: /Inventaire du Drive/ }),
		)
		expect(
			screen.getByText('Ouverture de « Inventaire du Drive »'),
		).toBeTruthy()
	})
})
