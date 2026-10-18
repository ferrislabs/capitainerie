import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { GithubPanel } from '#/domain/integrations/ui/github-panel'

const UNCONFIGURED: Schemas.GithubIntegrationResponse = {
	app_configured: false,
	connected: false,
}
const DISCONNECTED: Schemas.GithubIntegrationResponse = {
	app_configured: true,
	connected: false,
}
const CONNECTED: Schemas.GithubIntegrationResponse = {
	app_configured: true,
	connected: true,
	account_login: 'acme',
	connected_by: 'ada',
	connected_at: '2026-10-17T14:05:00Z',
	installation_id: 42,
}

function setup(
	status: Schemas.GithubIntegrationResponse,
	isAdmin: boolean,
	overrides: Partial<Parameters<typeof GithubPanel>[0]> = {},
) {
	const handlers = { onConnect: vi.fn(), onDisconnect: vi.fn() }
	render(
		<GithubPanel
			status={status}
			isAdmin={isAdmin}
			isConnecting={false}
			{...handlers}
			{...overrides}
		/>,
	)
	return handlers
}

describe('GithubPanel', () => {
	for (const isAdmin of [true, false]) {
		const role = isAdmin ? 'admin' : 'member'

		it(`explains the missing server configuration to a ${role}`, () => {
			setup(UNCONFIGURED, isAdmin)

			expect(screen.getByText(/pas configurée sur ce serveur/)).toBeTruthy()
			expect(screen.getByText('GITHUB_APP_*').className).toContain('font-mono')
			expect(screen.queryByRole('button')).toBeNull()
		})
	}

	it('lets an admin connect GitHub', async () => {
		const handlers = setup(DISCONNECTED, true)

		await userEvent.click(
			screen.getByRole('button', { name: 'Connecter GitHub' }),
		)

		expect(handlers.onConnect).toHaveBeenCalledOnce()
	})

	it('shows a member a read-only text when GitHub is not connected', () => {
		setup(DISCONNECTED, false)

		expect(screen.queryByRole('button')).toBeNull()
		expect(screen.getByText(/Seuls les administrateurs/)).toBeTruthy()
	})

	it('shows the connection error and locks the button while redirecting', () => {
		setup(DISCONNECTED, true, {
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

	it('shows the account, who connected it and when, and lets an admin disconnect', async () => {
		const handlers = setup(CONNECTED, true)

		expect(screen.getByText('acme').className).toContain('font-mono')
		expect(screen.getByText('ada')).toBeTruthy()
		expect(screen.getByText('17 octobre 2026')).toBeTruthy()
		await userEvent.click(screen.getByRole('button', { name: 'Déconnecter' }))
		expect(handlers.onDisconnect).toHaveBeenCalledOnce()
	})

	it('hides the disconnect action from a member', () => {
		setup(CONNECTED, false)

		expect(screen.getByText('acme')).toBeTruthy()
		expect(screen.queryByRole('button')).toBeNull()
	})
})
