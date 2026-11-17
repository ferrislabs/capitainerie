import { render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApprovalsPage } from '#/domain/approvals/feature/approvals-page'

afterEach(() => vi.restoreAllMocks())

describe('ApprovalsPage', () => {
	it('renders the empty state, banner and history without network', () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		render(<ApprovalsPage />)
		expect(screen.getByRole('heading', { name: 'Approbations' })).toBeTruthy()
		expect(screen.getByText(/démonstration/i)).toBeTruthy()
		expect(screen.getByText('Aucune approbation en attente')).toBeTruthy()
		expect(screen.getByText('Historique récent')).toBeTruthy()
		expect(screen.getAllByText('Approuvée')).toHaveLength(2)
		expect(screen.getAllByText('Refusée')).toHaveLength(1)
		expect(fetchSpy).not.toHaveBeenCalled()
	})
})
