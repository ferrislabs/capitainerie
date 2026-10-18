import { render, screen, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { StrictMode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { GithubAppCallbackPage } from '#/domain/integrations/feature/github-app-callback-page'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

const navigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
	Link: ({ to, children }: { to: string; children: ReactNode }) => (
		<a href={to}>{children}</a>
	),
	useNavigate: () => navigate,
}))

describe('GithubAppCallbackPage', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function mount(props: Parameters<typeof GithubAppCallbackPage>[0]) {
		render(
			<StrictMode>
				<GithubAppCallbackPage {...props} />
			</StrictMode>,
			{ wrapper: harness.wrapper },
		)
	}

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
	})

	afterEach(() => {
		fetchMock.mockReset()
		navigate.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it.each([
		['code', { state: 's' }],
		['state', { code: 'c' }],
	])('shows an error with a way back when %s is missing', (_name, props) => {
		mount(props)

		expect(screen.getByRole('alert').textContent).toContain('incomplet')
		expect(
			screen.getByRole('link', { name: 'Retour aux intégrations' }),
		).toBeTruthy()
		expect(fetchMock).not.toHaveBeenCalled()
	})

	it('posts the code once, even under strict mode, then goes back with a notice', async () => {
		fetchMock.mockResolvedValue(
			json(201, {
				slug: 'capi',
				owner_login: 'acme',
				owner_type: 'Organization',
				html_url: 'https://github.com/apps/capi',
				created_by: 'ada',
				created_at: '2026-10-18T10:00:00Z',
			}),
		)
		mount({ code: 'abc', state: 'signed' })

		await waitFor(() =>
			expect(navigate).toHaveBeenCalledWith({
				to: '/settings/integrations',
				search: { app: 'created' },
				replace: true,
			}),
		)
		expect(fetchMock).toHaveBeenCalledTimes(1)
		const [url, init] = fetchMock.mock.calls[0] as [URL, RequestInit]
		expect(String(url)).toBe('http://api.test/api/v1/integrations/github/app')
		expect(init.method).toBe('POST')
		expect(JSON.parse(String(init.body))).toEqual({
			code: 'abc',
			state: 'signed',
		})
	})

	it.each([
		[
			400,
			'invalid_state',
			'x',
			'Le lien de création a expiré, recommencez depuis les intégrations.',
		],
		[
			422,
			'invalid',
			'the manifest code is malformed',
			'GitHub a refusé la création de l’application (code expiré ou déjà utilisé). Recommencez depuis les intégrations.',
		],
		[
			409,
			'conflict',
			'github_app_exists',
			'Cette organisation a déjà une application GitHub.',
		],
		[
			403,
			'forbidden',
			'only an organization admin may do this',
			'Seuls les administrateurs de l’organisation peuvent gérer GitHub.',
		],
		[
			500,
			'internal',
			'boom',
			'La création de l’application GitHub a échoué. Recommencez.',
		],
	])('maps %s %s to a French message', async (status, code, message, expected) => {
		fetchMock.mockResolvedValue(json(status, { code, status, message }))
		mount({ code: 'abc', state: 'signed' })

		expect((await screen.findByRole('alert')).textContent).toBe(expected)
		expect(navigate).not.toHaveBeenCalled()
		expect(
			screen.getByRole('link', { name: 'Retour aux intégrations' }),
		).toBeTruthy()
	})
})
