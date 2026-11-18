import { screen, waitFor } from '@testing-library/react'
import { StrictMode } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import {
	GithubAppCallbackPage,
	GithubInstallCallbackPage,
} from '#/domain/integrations/feature/github-callback-pages'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'

const EXTRA = [
	'/settings/integrations/github/app-callback',
	'/settings/integrations/github/callback',
]

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

function posts(fetchMock: ReturnType<typeof mockApi>) {
	return fetchMock.mock.calls
		.filter(([, init]) => init?.method === 'POST')
		.map(([url, init]) => ({
			path: new URL(String(url)).pathname,
			body: JSON.parse(String(init?.body)),
		}))
}

async function mount(
	ui: React.ReactNode,
	routes: Parameters<typeof mockApi>[0],
) {
	const fetchMock = mockApi(routes)
	const view = await renderAtPath(
		<StrictMode>{withQueryClient(ui)}</StrictMode>,
		'/settings/integrations/github/callback',
		EXTRA,
	)
	return { fetchMock, router: view.router }
}

describe('GitHub app creation return', () => {
	it('exchanges the code once, then goes back to the GitHub page', async () => {
		const { fetchMock, router } = await mount(
			<GithubAppCallbackPage code="c1" state="s1" />,
			{
				'POST /api/v1/integrations/github/app': {
					status: 201,
					body: {
						slug: 'capitainerie-acme',
						owner_login: 'acme',
						owner_type: 'Organization',
						html_url: 'https://github.com/apps/capitainerie-acme',
						created_by: 'u',
						created_at: '2026-11-01T00:00:00Z',
					},
				},
			},
		)

		expect(screen.getByText("Enregistrement de l'application…")).toBeTruthy()
		await waitFor(() =>
			expect(router.state.location.pathname).toBe(
				'/settings/integrations/github',
			),
		)
		expect(router.state.location.search).toEqual({ retour: 'app' })
		expect(posts(fetchMock)).toEqual([
			{
				path: '/api/v1/integrations/github/app',
				body: { code: 'c1', state: 's1' },
			},
		])
	})

	it('goes back to the GitHub page when the app was already saved', async () => {
		const { router } = await mount(
			<GithubAppCallbackPage code="c1" state="s1" />,
			{
				'POST /api/v1/integrations/github/app': {
					status: 409,
					body: { code: 'github_app_exists', message: 'exists', status: 409 },
				},
			},
		)

		await waitFor(() =>
			expect(router.state.location.pathname).toBe(
				'/settings/integrations/github',
			),
		)
		expect(router.state.location.search).toEqual({ retour: 'app' })
	})

	it('explains an expired code', async () => {
		await mount(<GithubAppCallbackPage code="c1" state="s1" />, {
			'POST /api/v1/integrations/github/app': {
				status: 422,
				body: { code: 'invalid', message: 'bad code', status: 422 },
			},
		})

		expect(
			await screen.findByText(/GitHub a refusé le code : il a expiré/),
		).toBeTruthy()
		expect(screen.getByRole('button', { name: 'Retour à GitHub' })).toBeTruthy()
	})

	it('refuses a forged or expired state', async () => {
		await mount(<GithubAppCallbackPage code="c1" state="s1" />, {
			'POST /api/v1/integrations/github/app': {
				status: 400,
				body: { code: 'invalid_state', message: 'expired', status: 400 },
			},
		})

		expect(await screen.findByText(/Le lien a expiré/)).toBeTruthy()
	})

	it('does not call the API without a code', async () => {
		const { fetchMock } = await mount(
			<GithubAppCallbackPage code={undefined} state="s1" />,
			{},
		)

		expect(screen.getByText(/le code ou l'état est manquant/)).toBeTruthy()
		expect(posts(fetchMock)).toEqual([])
	})
})

describe('GitHub installation return', () => {
	it('connects the installation once, then goes back to the GitHub page', async () => {
		const { fetchMock, router } = await mount(
			<GithubInstallCallbackPage
				installationId={42}
				setupAction="install"
				code="c2"
				state="s2"
			/>,
			{
				'POST /api/v1/integrations/github/callback': {
					app: null,
					installation: {
						account_login: 'acme',
						installation_id: 42,
						connected_by: 'u',
						connected_at: '2026-11-01T00:00:00Z',
					},
				},
			},
		)

		await waitFor(() =>
			expect(router.state.location.search).toEqual({
				retour: 'installation',
			}),
		)
		expect(posts(fetchMock)).toEqual([
			{
				path: '/api/v1/integrations/github/callback',
				body: { installation_id: 42, code: 'c2', state: 's2' },
			},
		])
	})

	it('explains an installation the user does not administer', async () => {
		await mount(
			<GithubInstallCallbackPage
				installationId={42}
				setupAction="install"
				code="c2"
				state="s2"
			/>,
			{
				'POST /api/v1/integrations/github/callback': {
					status: 403,
					body: {
						code: 'installation_not_owned',
						message: 'nope',
						status: 403,
					},
				},
			},
		)

		expect(
			await screen.findByText(/Tu n'administres pas le compte GitHub/),
		).toBeTruthy()
	})

	it('says a requested installation waits for an owner', async () => {
		const { fetchMock } = await mount(
			<GithubInstallCallbackPage
				installationId={undefined}
				setupAction="request"
				code="c2"
				state={undefined}
			/>,
			{},
		)

		expect(
			screen.getByText(/Ta demande d'installation a été envoyée/),
		).toBeTruthy()
		expect(posts(fetchMock)).toEqual([])
	})

	it('reports an incomplete return', async () => {
		const { fetchMock } = await mount(
			<GithubInstallCallbackPage
				installationId={undefined}
				setupAction="install"
				code={undefined}
				state="s2"
			/>,
			{},
		)

		expect(screen.getByText(/Retour de GitHub incomplet/)).toBeTruthy()
		expect(posts(fetchMock)).toEqual([])
	})
})
