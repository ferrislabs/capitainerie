import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { GithubPanel } from '#/domain/integrations/ui/github-panel'

const APP: Schemas.GithubAppResponse = {
	slug: 'capitainerie-acme',
	owner_login: 'acme',
	owner_type: 'Organization',
	html_url: 'https://github.com/apps/capitainerie-acme',
	created_by: 'ada',
	created_at: '2026-10-16T09:00:00Z',
}
const NO_APP: Schemas.GithubIntegrationResponse = {
	app: null,
	installation: null,
}
const APP_ONLY: Schemas.GithubIntegrationResponse = {
	app: APP,
	installation: null,
}
const INSTALLED: Schemas.GithubIntegrationResponse = {
	app: APP,
	installation: {
		account_login: 'acme',
		connected_by: 'ada',
		connected_at: '2026-10-17T14:05:00Z',
		installation_id: 42,
	},
}

function setup(
	status: Schemas.GithubIntegrationResponse,
	isAdmin: boolean,
	overrides: Partial<Parameters<typeof GithubPanel>[0]> = {},
) {
	const handlers = {
		onConnect: vi.fn(),
		onDisconnect: vi.fn(),
		onDeleteApp: vi.fn(),
	}
	const create = {
		ownerKind: 'user' as const,
		login: '',
		isPending: false,
		onOwnerKindChange: vi.fn(),
		onLoginChange: vi.fn(),
		onSubmit: vi.fn(),
	}
	render(
		<GithubPanel
			status={status}
			isAdmin={isAdmin}
			create={create}
			isConnecting={false}
			{...handlers}
			{...overrides}
		/>,
	)
	return { ...handlers, create }
}

describe('GithubPanel without an App', () => {
	it('lets an admin create the App', async () => {
		const { create } = setup(NO_APP, true)

		await userEvent.click(
			screen.getByRole('button', { name: 'Créer l’application GitHub' }),
		)

		expect(create.onSubmit).toHaveBeenCalledOnce()
	})

	it('shows a member a read-only text', () => {
		setup(NO_APP, false)

		expect(screen.queryByRole('button')).toBeNull()
		expect(screen.getByText(/pas encore d’application GitHub/)).toBeTruthy()
		expect(screen.queryByText(/configurée sur ce serveur/)).toBeNull()
	})
})

describe('GithubPanel with an App but no installation', () => {
	it('links the App and lets an admin install it', async () => {
		const handlers = setup(APP_ONLY, true)

		const link = screen.getByRole('link', { name: 'capitainerie-acme' })
		expect(link.getAttribute('href')).toBe(APP.html_url)
		await userEvent.click(
			screen.getByRole('button', { name: 'Installer sur vos dépôts' }),
		)
		expect(handlers.onConnect).toHaveBeenCalledOnce()
		await userEvent.click(
			screen.getByRole('button', { name: 'Supprimer l’application' }),
		)
		expect(handlers.onDeleteApp).toHaveBeenCalledOnce()
	})

	it('shows a member a read-only text', () => {
		setup(APP_ONLY, false)

		expect(screen.queryByRole('button')).toBeNull()
		expect(screen.getByText(/Seuls les administrateurs/)).toBeTruthy()
	})

	it('shows the install error and locks the button while redirecting', () => {
		setup(APP_ONLY, true, {
			isConnecting: true,
			errorMessage: 'Impossible de démarrer la connexion à GitHub.',
		})

		expect(
			(
				screen.getByRole('button', {
					name: 'Redirection…',
				}) as HTMLButtonElement
			).disabled,
		).toBe(true)
		expect(screen.getByRole('alert').textContent).toBe(
			'Impossible de démarrer la connexion à GitHub.',
		)
	})
})

describe('GithubPanel installed', () => {
	it('shows the account, the App, and lets an admin disconnect or delete', async () => {
		const handlers = setup(INSTALLED, true)

		expect(screen.getByText('acme', { selector: 'dd' }).className).toContain(
			'font-mono',
		)
		expect(screen.getByText('ada')).toBeTruthy()
		expect(screen.getByText('17 octobre 2026')).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'capitainerie-acme' })
				.getAttribute('href'),
		).toBe(APP.html_url)
		await userEvent.click(screen.getByRole('button', { name: 'Déconnecter' }))
		expect(handlers.onDisconnect).toHaveBeenCalledOnce()
		await userEvent.click(
			screen.getByRole('button', { name: 'Supprimer l’application' }),
		)
		expect(handlers.onDeleteApp).toHaveBeenCalledOnce()
	})

	it('hides every action from a member', () => {
		setup(INSTALLED, false)

		expect(screen.getByText('acme', { selector: 'dd' })).toBeTruthy()
		expect(screen.queryByRole('button')).toBeNull()
	})
})
