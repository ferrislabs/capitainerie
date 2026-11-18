import { screen, waitFor } from '@testing-library/react'
import { StrictMode } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { OAuthCallbackPage } from '#/domain/connectors/feature/oauth-callback-page'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'

const EXTRA = ['/connectors/{-$tab}', '/connectors/oauth/callback']

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

function posts(fetchMock: ReturnType<typeof mockApi>) {
	return fetchMock.mock.calls.filter(([, init]) => init?.method === 'POST')
}

async function open(
	props: Parameters<typeof OAuthCallbackPage>[0],
	routes: Parameters<typeof mockApi>[0] = {},
) {
	const fetchMock = mockApi(routes)
	await renderAtPath(
		<StrictMode>
			{withQueryClient(<OAuthCallbackPage {...props} />)}
		</StrictMode>,
		'/connectors/oauth/callback',
		EXTRA,
	)
	return fetchMock
}

describe('OAuthCallbackPage', () => {
	it('posts code and state once, even under strict mode, then leaves', async () => {
		const fetchMock = await open(
			{ code: 'c1', state: 's1', error: undefined },
			{
				'POST /api/v1/mcp-connectors/oauth/callback': {
					connector_id: 'mestier',
				},
			},
		)
		expect(screen.getByText('Finalisation de la connexion…')).toBeTruthy()
		await waitFor(() => expect(posts(fetchMock)).toHaveLength(1))
		expect(screen.queryByRole('alert')).toBeNull()
		expect(JSON.parse(String(posts(fetchMock)[0]?.[1]?.body))).toEqual({
			code: 'c1',
			state: 's1',
		})
	})

	it('shows an error when the callback is rejected', async () => {
		const fetchMock = await open(
			{ code: 'c1', state: 's1', error: undefined },
			{
				'POST /api/v1/mcp-connectors/oauth/callback': {
					status: 422,
					body: { code: 'invalid', message: 'x', status: 422 },
				},
			},
		)
		expect(await screen.findByText(/invalide ou a expiré/)).toBeTruthy()
		expect(
			screen.getByRole('button', { name: 'Retour aux connecteurs' }),
		).toBeTruthy()
		expect(posts(fetchMock)).toHaveLength(1)
	})

	it('shows the IdP error without calling the API', async () => {
		const fetchMock = await open({
			code: undefined,
			state: 's1',
			error: 'access_denied',
		})
		expect(await screen.findByText(/Autorisation refusée/)).toBeTruthy()
		expect(posts(fetchMock)).toHaveLength(0)
	})

	it('rejects a return without code', async () => {
		const fetchMock = await open({
			code: undefined,
			state: undefined,
			error: undefined,
		})
		expect(await screen.findByText(/incomplet/)).toBeTruthy()
		expect(posts(fetchMock)).toHaveLength(0)
	})
})
