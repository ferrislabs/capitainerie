import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ArtifactsPage } from '#/domain/artifacts/feature/artifacts-page'

afterEach(() => vi.restoreAllMocks())

describe('ArtifactsPage', () => {
	it('renders title, banner and the list without fetching', () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		render(<ArtifactsPage />)
		expect(screen.getByRole('heading', { name: 'Artifacts' })).toBeTruthy()
		expect(screen.getByText(/Données de démonstration/)).toBeTruthy()
		expect(screen.getAllByRole('row')).toHaveLength(7)
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('filters with the search field and shows an empty state', async () => {
		render(<ArtifactsPage />)
		const search = screen.getByRole('searchbox', {
			name: 'Rechercher un artifact',
		})
		await userEvent.type(search, 'drive')
		expect(screen.getAllByRole('row')).toHaveLength(2)
		await userEvent.clear(search)
		await userEvent.type(search, 'zzz')
		expect(screen.getByText('Aucun artifact')).toBeTruthy()
	})
})
