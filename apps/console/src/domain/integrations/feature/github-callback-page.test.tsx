import { render, screen, waitFor } from '@testing-library/react'
import type { ReactNode } from 'react'
import { StrictMode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { GithubCallbackPage } from '#/domain/integrations/feature/github-callback-page'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

const navigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
	Link: ({ to, children }: { to: string; children: ReactNode }) => (
		<a href={to}>{children}</a>
	),
	useNavigate: () => navigate,
}))

describe('GithubCallbackPage', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function mount(props: Parameters<typeof GithubCallbackPage>[0]) {
		render(
			<StrictMode>
				<GithubCallbackPage {...props} />
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
		['installation_id', { code: 'c', state: 's' }],
		['code', { installationId: '42', state: 's' }],
		['state', { installationId: '42', code: 'c' }],
		[
			'a numeric installation_id',
			{ installationId: 'abc', code: 'c', state: 's' },
		],
	])('shows an error with a way back when %s is missing', (_name, props) => {
		mount(props)

		expect(screen.getByRole('alert').textContent).toContain('incomplet')
		expect(
			screen.getByRole('link', { name: 'Retour aux intégrations' }),
		).toBeTruthy()
		expect(fetchMock).not.toHaveBeenCalled()
	})

	it('posts the callback once, even under strict mode, then goes back with a notice', async () => {
		fetchMock.mockResolvedValue(json(200, { app: null, installation: null }))
		mount({ installationId: '42', code: 'abc', state: 'signed' })

		await waitFor(() =>
			expect(navigate).toHaveBeenCalledWith({
				to: '/settings/integrations',
				search: { connected: true },
				replace: true,
			}),
		)
		expect(fetchMock).toHaveBeenCalledTimes(1)
		const [url, init] = fetchMock.mock.calls[0] as [URL, RequestInit]
		expect(String(url)).toBe(
			'http://api.test/api/v1/integrations/github/callback',
		)
		expect(JSON.parse(String(init.body))).toEqual({
			installation_id: 42,
			code: 'abc',
			state: 'signed',
		})
	})

	it.each([
		[400, 'invalid_state', 'x', 'Le lien de connexion a expiré, recommencez'],
		[
			403,
			'installation_not_accessible',
			'x',
			'Vous devez être propriétaire du compte GitHub qui a installé l’application',
		],
		[
			403,
			'installation_not_owned',
			'x',
			'Vous devez être propriétaire du compte GitHub qui a installé l’application',
		],
		[
			409,
			'conflict',
			'installation_already_connected',
			'Cette installation GitHub est déjà connectée à une autre organisation.',
		],
		[
			409,
			'conflict',
			'github_app_missing',
			'Créez d’abord l’application GitHub de l’organisation.',
		],
	])('maps %s %s to a French message', async (status, code, message, expected) => {
		fetchMock.mockResolvedValue(json(status, { code, status, message }))
		mount({ installationId: '42', code: 'abc', state: 'signed' })

		expect((await screen.findByRole('alert')).textContent).toBe(expected)
		expect(navigate).not.toHaveBeenCalled()
		expect(
			screen.getByRole('link', { name: 'Retour aux intégrations' }),
		).toBeTruthy()
	})
})
