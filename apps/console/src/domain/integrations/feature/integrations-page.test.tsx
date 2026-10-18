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

const APP = {
	slug: 'capitainerie-acme',
	owner_login: 'acme',
	owner_type: 'Organization',
	html_url: 'https://github.com/apps/capitainerie-acme',
	created_by: 'ada',
	created_at: '2026-10-16T09:00:00Z',
}
const NO_APP = { app: null, installation: null }
const APP_ONLY = { app: APP, installation: null }
const CONNECTED = {
	app: APP,
	installation: {
		account_login: 'acme',
		connected_by: 'ada',
		connected_at: '2026-10-17T14:05:00Z',
		installation_id: 42,
	},
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

	it('installs the App: redirects an admin to GitHub', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		fetchMock.mockImplementation((url: URL) =>
			Promise.resolve(
				url.pathname.endsWith('/install')
					? json(200, { url: 'https://github.com/apps/capi/installations/new' })
					: json(200, APP_ONLY),
			),
		)
		render(<IntegrationsPage />, { wrapper: harness.wrapper })

		await userEvent.click(
			await screen.findByRole('button', { name: 'Installer sur vos dépôts' }),
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
				return Promise.resolve(json(200, APP_ONLY))
			}
			return Promise.resolve(json(200, connected ? CONNECTED : APP_ONLY))
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
			await screen.findByRole('button', { name: 'Installer sur vos dépôts' }),
		).toBeTruthy()
	})

	it('creates the App: asks for the manifest, then posts it to GitHub', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		const submit = vi
			.spyOn(HTMLFormElement.prototype, 'submit')
			.mockImplementation(() => undefined)
		fetchMock.mockImplementation((url: URL, init: RequestInit) =>
			Promise.resolve(
				init.method === 'POST' && url.pathname.endsWith('/app/manifest')
					? json(200, {
							action_url:
								'https://github.com/organizations/acme/settings/apps/new?state=s.t',
							manifest: '{"name":"Capitainerie Test 1a2b3c"}',
							state: 's.t',
						})
					: json(200, NO_APP),
			),
		)
		const user = userEvent.setup()
		render(<IntegrationsPage />, { wrapper: harness.wrapper })

		await user.click(await screen.findByLabelText('Organisation GitHub'))
		await user.type(screen.getByLabelText(/Login de l’organisation/), ' acme ')
		await user.click(
			screen.getByRole('button', { name: 'Créer l’application GitHub' }),
		)

		const form = (await screen.findByTestId(
			'github-manifest-form',
		)) as HTMLFormElement
		expect(form.getAttribute('action')).toBe(
			'https://github.com/organizations/acme/settings/apps/new?state=s.t',
		)
		expect(
			(form.elements.namedItem('manifest') as HTMLInputElement).value,
		).toBe('{"name":"Capitainerie Test 1a2b3c"}')
		expect(submit).toHaveBeenCalledTimes(1)
		const post = fetchMock.mock.calls.find(
			([url, init]) =>
				init.method === 'POST' && url.pathname.endsWith('/app/manifest'),
		)
		expect(JSON.parse(String(post?.[1].body))).toEqual({
			owner: { type: 'organization', login: 'acme' },
		})
		submit.mockRestore()
	})

	it('creates the App for a personal account without a login', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		const submit = vi
			.spyOn(HTMLFormElement.prototype, 'submit')
			.mockImplementation(() => undefined)
		fetchMock.mockImplementation((_url: URL, init: RequestInit) =>
			Promise.resolve(
				init.method === 'POST'
					? json(200, {
							action_url: 'https://github.com/settings/apps/new?state=s.t',
							manifest: '{}',
							state: 's.t',
						})
					: json(200, NO_APP),
			),
		)
		render(<IntegrationsPage />, { wrapper: harness.wrapper })

		await userEvent.click(
			await screen.findByRole('button', { name: 'Créer l’application GitHub' }),
		)

		await screen.findByTestId('github-manifest-form')
		const post = fetchMock.mock.calls.find(([, init]) => init.method === 'POST')
		expect(JSON.parse(String(post?.[1].body))).toEqual({
			owner: { type: 'user' },
		})
		submit.mockRestore()
	})

	it('says when the server cannot start the creation', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		fetchMock.mockImplementation((_url: URL, init: RequestInit) =>
			Promise.resolve(
				init.method === 'POST'
					? json(503, {
							code: 'console_url_not_configured',
							status: 503,
							message: 'CONSOLE_URL is unset',
						})
					: json(200, NO_APP),
			),
		)
		render(<IntegrationsPage />, { wrapper: harness.wrapper })

		await userEvent.click(
			await screen.findByRole('button', { name: 'Créer l’application GitHub' }),
		)

		expect((await screen.findByRole('alert')).textContent).toContain(
			'CONSOLE_URL',
		)
		expect(screen.queryByTestId('github-manifest-form')).toBeNull()
	})

	it('deletes the App after an explanation, then points to GitHub', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		let deleted = false
		fetchMock.mockImplementation((_url: URL, init: RequestInit) => {
			if (init.method === 'DELETE') {
				deleted = true
				return Promise.resolve(json(200, APP))
			}
			return Promise.resolve(json(200, deleted ? NO_APP : CONNECTED))
		})
		const user = userEvent.setup()
		render(<IntegrationsPage />, { wrapper: harness.wrapper })

		await user.click(
			await screen.findByRole('button', { name: 'Supprimer l’application' }),
		)
		const dialog = await screen.findByRole('alertdialog')
		expect(within(dialog).getByText(/reste sur GitHub/)).toBeTruthy()
		expect(
			fetchMock.mock.calls.some(([, init]) => init.method === 'DELETE'),
		).toBe(false)
		await user.click(
			within(dialog).getByRole('button', { name: 'Supprimer l’application' }),
		)

		const notice = await screen.findByText(/supprimée de Capitainerie/)
		expect(
			within(notice.closest('output') as HTMLElement)
				.getByRole('link')
				.getAttribute('href'),
		).toBe(APP.html_url)
		expect(
			await screen.findByRole('button', { name: 'Créer l’application GitHub' }),
		).toBeTruthy()
		const call = fetchMock.mock.calls.find(
			([, init]) => init.method === 'DELETE',
		)
		expect(call?.[0].pathname).toBe('/api/v1/integrations/github/app')
	})

	it('tells the admin the App was just created', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		fetchMock.mockResolvedValue(json(200, APP_ONLY))
		render(<IntegrationsPage appCreated />, { wrapper: harness.wrapper })

		expect(
			await screen.findByText(/L’application GitHub est créée/),
		).toBeTruthy()
		expect(
			await screen.findByRole('button', { name: 'Installer sur vos dépôts' }),
		).toBeTruthy()
	})

	it('no longer mentions a server-side configuration', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		fetchMock.mockResolvedValue(json(200, NO_APP))
		render(<IntegrationsPage />, { wrapper: harness.wrapper })

		await screen.findByRole('button', { name: 'Créer l’application GitHub' })
		expect(screen.queryByText(/configurée sur ce serveur/)).toBeNull()
	})

	it('is read-only for a member and shows the connected account', async () => {
		setAuth({ accessToken: tokenWithRoles(['member']) })
		fetchMock.mockResolvedValue(json(200, CONNECTED))
		render(<IntegrationsPage justConnected />, { wrapper: harness.wrapper })

		expect(await screen.findByText('acme', { selector: 'dd' })).toBeTruthy()
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
