import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { SkillsPage } from '#/domain/skills/feature/skills-page'

afterEach(() => vi.restoreAllMocks())

describe('SkillsPage', () => {
	it('renders title, banner and the 6 skills without network', () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		render(<SkillsPage />)
		expect(
			screen.getByRole('heading', { name: 'Compétences', level: 1 }),
		).toBeTruthy()
		expect(screen.getByText(/démonstration/i)).toBeTruthy()
		expect(screen.getAllByRole('listitem')).toHaveLength(6)
		expect(screen.getAllByText('Intégré')).toHaveLength(2)
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('filters by name', async () => {
		render(<SkillsPage />)
		await userEvent.type(
			screen.getByRole('searchbox', { name: 'Rechercher une compétence' }),
			'ui-',
		)
		expect(screen.getAllByRole('listitem')).toHaveLength(1)
		expect(screen.getByText('ui-ux')).toBeTruthy()
	})

	it('shows a notice on create', async () => {
		render(<SkillsPage />)
		await userEvent.click(
			screen.getByRole('button', { name: 'Nouvelle compétence' }),
		)
		expect(screen.getByText('Importer ou écrire une compétence')).toBeTruthy()
	})
})
