import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { ADMIN_ROLE } from '#/domain/identity/roles'
import { IntegrationsPage } from '#/domain/integrations/feature/integrations-page'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

function tokenWithRoles(roles: Array<string>) {
	const payload = btoa(JSON.stringify({ realm_access: { roles } }))
	return `e30.${payload.replace(/=+$/, '')}.sig`
}

const CONNECTED = {
	app_configured: true,
	connected: true,
	account_login: 'acme',
	connected_by: 'ada',
	connected_at: '2026-10-17T14:05:00Z',
}

describe('IntegrationsPage', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>
	const assign = vi.fn()
	const originalLocation = window.location

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		Object.defineProperty(window, 'location', {
			configurable: true,
			value: { ...originalLocation, assign },
		})
	})

	afterEach(() => {
		fetchMock.mockReset()
		assign.mockReset()
		vi.unstubAllGlobals()
		Object.defineProperty(window, 'location', {
			configurable: true,
			value: originalLocation,
		})
		clearAuth()
		harness.client.clear()
	})

	it('starts the installation and redirects to GitHub for an admin', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		fetchMock.mockImplementation((url: URL) =>
			Promise.resolve(
				url.pathname.endsWith('/install')
					? json(200, { url: 'https://github.com/apps/capi/installations/new' })
					: json(200, { app_configured: true, connected: false }),
			),
		)
		render(<IntegrationsPage />, { wrapper: harness.wrapper })

		await userEvent.click(
			await screen.findByRole('button', { name: 'Connecter GitHub' }),
		)

		await waitFor(() =>
			expect(assign).toHaveBeenCalledWith(
				'https://github.com/apps/capi/installations/new',
			),
		)
	})

	it('confirms before disconnecting, then reports it', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		let connected = true
		fetchMock.mockImplementation((_url: URL, init: RequestInit) => {
			if (init.method === 'DELETE') {
				connected = false
				return Promise.resolve(json(200, { app_configured: true, connected }))
			}
			return Promise.resolve(
				json(200, connected ? CONNECTED : { app_configured: true, connected }),
			)
		})
		const user = userEvent.setup()
		render(<IntegrationsPage />, { wrapper: harness.wrapper })

		await user.click(await screen.findByRole('button', { name: 'Déconnecter' }))
		const dialog = await screen.findByRole('alertdialog')
		expect(
			fetchMock.mock.calls.some(([, init]) => init.method === 'DELETE'),
		).toBe(false)
		await user.click(
			within(dialog).getByRole('button', { name: 'Déconnecter' }),
		)

		expect(await screen.findByText('GitHub est déconnecté.')).toBeTruthy()
		expect(
			await screen.findByRole('button', { name: 'Connecter GitHub' }),
		).toBeTruthy()
	})

	it('is read-only for a member and shows the connected account', async () => {
		setAuth({ accessToken: tokenWithRoles(['member']) })
		fetchMock.mockResolvedValue(json(200, CONNECTED))
		render(<IntegrationsPage justConnected />, { wrapper: harness.wrapper })

		expect(await screen.findByText('acme')).toBeTruthy()
		expect(screen.getByText('GitHub est connecté.')).toBeTruthy()
		expect(screen.queryByRole('button')).toBeNull()
	})

	it('shows an error when the state cannot be loaded', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		fetchMock.mockResolvedValue(
			json(500, { code: 'internal', status: 500, message: 'boom' }),
		)
		render(<IntegrationsPage />, { wrapper: harness.wrapper })

		expect(
			await screen.findByText(/Impossible de charger l’état de la connexion/),
		).toBeTruthy()
	})
})
