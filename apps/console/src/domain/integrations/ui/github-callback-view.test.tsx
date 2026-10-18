import { render, screen } from '@testing-library/react'
import type { ReactNode } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { GithubCallbackView } from '#/domain/integrations/ui/github-callback-view'

vi.mock('@tanstack/react-router', () => ({
	Link: ({ to, children }: { to: string; children: ReactNode }) => (
		<a href={to}>{children}</a>
	),
}))

describe('GithubCallbackView', () => {
	it('shows progress without an error', () => {
		render(<GithubCallbackView />)

		expect(screen.getByText('Connexion à GitHub en cours…')).toBeTruthy()
		expect(screen.queryByRole('link')).toBeNull()
	})

	it('shows the error with a way back', () => {
		render(<GithubCallbackView errorMessage="Le lien a expiré" />)

		expect(screen.getByRole('alert').textContent).toBe('Le lien a expiré')
		expect(
			screen
				.getByRole('link', { name: 'Retour aux intégrations' })
				.getAttribute('href'),
		).toBe('/settings/integrations')
	})
})
