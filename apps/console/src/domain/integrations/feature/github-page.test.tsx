import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { GithubPage } from '#/domain/integrations/feature/github-page'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'

const APP = {
	slug: 'capitainerie-acme-1a2b3c',
	owner_login: 'acme',
	owner_type: 'Organization',
	html_url: 'https://github.com/apps/capitainerie-acme-1a2b3c',
	created_by: 'u',
	created_at: '2026-11-01T00:00:00Z',
}
const INSTALLATION = {
	account_login: 'acme',
	installation_id: 7,
	connected_by: 'alice',
	connected_at: '2026-11-02T00:00:00Z',
}

async function open(routes: Parameters<typeof mockApi>[0], banner?: string) {
	const fetchMock = mockApi(routes)
	await renderAtPath(
		withQueryClient(<GithubPage banner={banner} />),
		'/settings/integrations/github',
	)
	return fetchMock
}

function calls(fetchMock: ReturnType<typeof mockApi>, method: string) {
	return fetchMock.mock.calls
		.filter(([, init]) => (init?.method ?? 'GET') === method)
		.map(([url, init]) => ({
			path: new URL(String(url)).pathname,
			body: init?.body ? JSON.parse(String(init.body)) : undefined,
		}))
}

afterEach(() => {
	vi.restoreAllMocks()
	vi.unstubAllGlobals()
	clearAuth()
	for (const form of document.querySelectorAll('form[action]')) form.remove()
})

describe('GitHub settings page', () => {
	it('creates the app through the manifest flow for an organization', async () => {
		const user = userEvent.setup()
		const submit = vi
			.spyOn(HTMLFormElement.prototype, 'submit')
			.mockImplementation(() => {})
		const fetchMock = await open({
			'GET /api/v1/integrations/github': { app: null, installation: null },
			'POST /api/v1/integrations/github/app/manifest': {
				action_url:
					'https://github.com/organizations/acme/settings/apps/new?state=s1',
				manifest: '{"name":"Capitainerie Acme"}',
				state: 's1',
			},
		})

		expect(
			await screen.findByText(/Étape 1 · Créer l'application GitHub/),
		).toBeTruthy()
		expect(
			(
				screen.getByRole('button', {
					name: 'Installer sur GitHub',
				}) as HTMLButtonElement
			).disabled,
		).toBe(true)

		await user.type(
			screen.getByRole('textbox', { name: /Login de l'organisation/ }),
			'acme',
		)
		await user.click(
			screen.getByRole('button', { name: "Créer l'application GitHub" }),
		)

		await waitFor(() => expect(submit).toHaveBeenCalledTimes(1))
		expect(calls(fetchMock, 'POST')).toEqual([
			{
				path: '/api/v1/integrations/github/app/manifest',
				body: { owner: { type: 'organization', login: 'acme' } },
			},
		])
		const form = submit.mock.contexts[0] as HTMLFormElement
		expect(form.getAttribute('action')).toBe(
			'https://github.com/organizations/acme/settings/apps/new?state=s1',
		)
		expect(form.method).toBe('post')
		expect(
			(form.querySelector('input[name="manifest"]') as HTMLInputElement).value,
		).toBe('{"name":"Capitainerie Acme"}')
	})

	it('asks for a valid organization login before calling the API', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			'GET /api/v1/integrations/github': { app: null, installation: null },
		})

		await user.click(
			await screen.findByRole('button', {
				name: "Créer l'application GitHub",
			}),
		)

		expect(
			await screen.findByText(/Indique le login de l'organisation GitHub/),
		).toBeTruthy()
		expect(calls(fetchMock, 'POST')).toEqual([])
	})

	it('creates the app on a personal account without a login', async () => {
		const user = userEvent.setup()
		vi.spyOn(HTMLFormElement.prototype, 'submit').mockImplementation(() => {})
		const fetchMock = await open({
			'GET /api/v1/integrations/github': { app: null, installation: null },
			'POST /api/v1/integrations/github/app/manifest': {
				action_url: 'https://github.com/settings/apps/new?state=s1',
				manifest: '{}',
				state: 's1',
			},
		})

		await user.click(
			await screen.findByRole('radio', { name: 'Mon compte GitHub personnel' }),
		)
		await user.click(
			screen.getByRole('button', { name: "Créer l'application GitHub" }),
		)

		await waitFor(() =>
			expect(calls(fetchMock, 'POST')[0]?.body).toEqual({
				owner: { type: 'user' },
			}),
		)
	})

	it('explains a refusal to a member who is not an admin', async () => {
		const user = userEvent.setup()
		await open({
			'GET /api/v1/integrations/github': { app: null, installation: null },
			'POST /api/v1/integrations/github/app/manifest': {
				status: 403,
				body: { code: 'forbidden', message: 'forbidden', status: 403 },
			},
		})

		await user.type(
			await screen.findByRole('textbox', { name: /Login de l'organisation/ }),
			'acme',
		)
		await user.click(
			screen.getByRole('button', { name: "Créer l'application GitHub" }),
		)

		expect(
			await screen.findByText(
				"Seul un administrateur de l'organisation peut configurer GitHub.",
			),
		).toBeTruthy()
	})

	it('sends the admin to the installation page once the app exists', async () => {
		const user = userEvent.setup()
		const assign = vi.fn()
		vi.stubGlobal('location', { ...window.location, assign })
		await open(
			{
				'GET /api/v1/integrations/github': { app: APP, installation: null },
				'GET /api/v1/integrations/github/install': {
					url: 'https://github.com/apps/capitainerie-acme-1a2b3c/installations/new?state=s2',
				},
			},
			'Application GitHub créée. Installe-la maintenant sur GitHub.',
		)

		expect(
			await screen.findByText(
				'Application GitHub créée. Installe-la maintenant sur GitHub.',
			),
		).toBeTruthy()
		expect(screen.getByText(APP.slug)).toBeTruthy()
		await user.click(
			screen.getByRole('button', { name: 'Installer sur GitHub' }),
		)

		await waitFor(() =>
			expect(assign).toHaveBeenCalledWith(
				'https://github.com/apps/capitainerie-acme-1a2b3c/installations/new?state=s2',
			),
		)
	})

	it('shows the connected account, its repositories, and disconnects', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			'GET /api/v1/integrations/github': {
				app: APP,
				installation: INSTALLATION,
			},
			'GET /api/v1/integrations/github/repositories': {
				repositories: [
					{
						owner: 'acme',
						name: 'web',
						full_name: 'acme/web',
						private: true,
						html_url: 'https://github.com/acme/web',
					},
				],
			},
			'DELETE /api/v1/integrations/github': { app: APP, installation: null },
		})

		expect(await screen.findByText('GitHub connecté')).toBeTruthy()
		expect(screen.getByText(/connectée par alice/)).toBeTruthy()
		const section = (
			await screen.findByRole('heading', { name: 'Dépôts GitHub accessibles' })
		).closest('section') as HTMLElement
		expect(await within(section).findByText('acme/web')).toBeTruthy()

		await user.click(screen.getByRole('button', { name: 'Déconnecter' }))
		const dialog = await screen.findByRole('alertdialog')
		await user.click(
			within(dialog).getByRole('button', { name: 'Déconnecter' }),
		)

		await waitFor(() =>
			expect(calls(fetchMock, 'DELETE')).toEqual([
				{ path: '/api/v1/integrations/github', body: undefined },
			]),
		)
		expect(await screen.findByText('GitHub déconnecté')).toBeTruthy()
	})
})
