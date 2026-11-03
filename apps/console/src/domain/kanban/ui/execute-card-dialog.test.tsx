import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { describe, expect, it, vi } from 'vitest'
import {
	ExecuteCardDialog,
	type RepositoriesState,
} from '#/domain/kanban/ui/execute-card-dialog'
import { chooseValue } from '#/test-utils/combobox'

vi.mock('@tanstack/react-router', () => ({
	Link: ({ to, children }: { to: string; children: ReactNode }) => (
		<a href={to}>{children}</a>
	),
}))

const READY: RepositoriesState = {
	kind: 'ready',
	repositories: [
		{ full_name: 'acme/web', name: 'web', owner: 'acme' },
		{ full_name: 'acme/api', name: 'api', owner: 'acme' },
	],
}

function setup(
	overrides: Partial<Parameters<typeof ExecuteCardDialog>[0]> = {},
) {
	const handlers = {
		onRepositoryChange: vi.fn(),
		onRetry: vi.fn(),
		onSubmit: vi.fn(),
		onClose: vi.fn(),
	}
	render(
		<ExecuteCardDialog
			open
			cardTitle="Corriger le bug"
			repositories={READY}
			repository=""
			isPending={false}
			{...handlers}
			{...overrides}
		/>,
	)
	return handlers
}

describe('ExecuteCardDialog', () => {
	it('lists the repositories and submits the chosen one', async () => {
		const handlers = setup({ repository: 'acme/web' })
		const user = userEvent.setup()

		await chooseValue(user, screen.getByLabelText('Dépôt'), 'acme/api')
		expect(handlers.onRepositoryChange).toHaveBeenCalledWith('acme/api')

		await user.click(screen.getByRole('button', { name: 'Confier' }))
		expect(handlers.onSubmit).toHaveBeenCalledOnce()
	})

	it('keeps Confier disabled until a repository is chosen', () => {
		setup()

		expect(
			(screen.getByRole('button', { name: 'Confier' }) as HTMLButtonElement)
				.disabled,
		).toBe(true)
	})

	it('always shows the isolation and GitHub Actions warning', () => {
		setup()

		const warning = screen.getByRole('status').textContent ?? ''
		expect(warning).toContain('conteneur isolé')
		expect(warning).toContain('capitainerie/…')
		expect(warning).toContain('pull request en brouillon')
		expect(warning).toContain('workflows GitHub Actions')
		expect(warning).toContain('secrets du dépôt')
		expect(warning).toContain('limitez-les aux branches protégées')
	})

	it('shows a loading state', () => {
		setup({ repositories: { kind: 'loading' } })

		expect(screen.getByText('Chargement des dépôts…')).toBeTruthy()
		expect(screen.queryByLabelText('Dépôt')).toBeNull()
	})

	it('explains an empty installation', () => {
		setup({ repositories: { kind: 'ready', repositories: [] } })

		expect(screen.getByText(/aucun dépôt/)).toBeTruthy()
		expect(screen.queryByLabelText('Dépôt')).toBeNull()
	})

	it('offers a retry on a loading error', async () => {
		const handlers = setup({ repositories: { kind: 'error' } })

		expect(screen.getByRole('alert').textContent).toBe(
			'Impossible de charger les dépôts GitHub.',
		)
		await userEvent.click(screen.getByRole('button', { name: 'Réessayer' }))
		expect(handlers.onRetry).toHaveBeenCalledOnce()
	})

	it('replaces the form with a link to the integrations when GitHub is not connected', () => {
		setup({ repositories: { kind: 'not_connected' } })

		expect(
			screen
				.getByRole('link', { name: 'Paramètres › Intégrations' })
				.getAttribute('href'),
		).toBe('/settings/integrations')
		expect(screen.queryByLabelText('Dépôt')).toBeNull()
		expect(screen.queryByRole('button', { name: 'Confier' })).toBeNull()
	})

	it('shows the mapped error and locks the form while pending', () => {
		setup({
			repository: 'acme/web',
			errorMessage: 'Une exécution est déjà en cours pour cette carte.',
			isPending: true,
		})

		expect(screen.getByRole('alert').textContent).toBe(
			'Une exécution est déjà en cours pour cette carte.',
		)
		expect(
			(screen.getByRole('button', { name: 'Envoi…' }) as HTMLButtonElement)
				.disabled,
		).toBe(true)
		expect((screen.getByLabelText('Dépôt') as HTMLSelectElement).disabled).toBe(
			true,
		)
	})

	it('closes from the cancel button', async () => {
		const handlers = setup()

		await userEvent.click(screen.getByRole('button', { name: 'Annuler' }))

		expect(handlers.onClose).toHaveBeenCalledOnce()
	})
})
