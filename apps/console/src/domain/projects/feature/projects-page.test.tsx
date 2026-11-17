import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it } from 'vitest'
import { ProjectsPage } from '#/domain/projects/feature/projects-page'

describe('ProjectsPage', () => {
	it('renders the five project cards with progress and counts', () => {
		render(<ProjectsPage />)
		expect(screen.getByRole('heading', { name: 'Projets' })).toBeTruthy()
		expect(screen.getByText('5 projets actifs')).toBeTruthy()
		expect(screen.getAllByRole('article')).toHaveLength(5)
		expect(screen.getByText('34 % · 12 ouvertes')).toBeTruthy()
		expect(
			screen
				.getByRole('progressbar', { name: 'Avancement de Conformité' })
				.getAttribute('aria-valuenow'),
		).toBe('82')
	})

	it('confirms the creation request', async () => {
		render(<ProjectsPage />)
		await userEvent.click(
			screen.getByRole('button', { name: 'Nouveau projet' }),
		)
		expect(screen.getByText('Création de projet')).toBeTruthy()
	})
})
