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

	it('toggles a skill', async () => {
		render(<SkillsPage />)
		const toggle = screen.getByRole('switch', { name: 'Activer ui-ux' })
		expect(toggle.getAttribute('aria-checked')).toBe('true')
		await userEvent.click(toggle)
		expect(toggle.getAttribute('aria-checked')).toBe('false')
	})

	it('shows a notice on create and new folder', async () => {
		render(<SkillsPage />)
		await userEvent.click(screen.getByRole('button', { name: 'Nouvelle' }))
		expect(screen.getByText('Importer ou écrire une compétence')).toBeTruthy()
		await userEvent.click(
			screen.getByRole('button', { name: 'Nouveau dossier' }),
		)
		expect(screen.getAllByText('Nouveau dossier')).toHaveLength(2)
	})
})
