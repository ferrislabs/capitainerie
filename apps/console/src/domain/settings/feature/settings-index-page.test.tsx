import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { renderAtPath } from '#/components/layout/test-router'
import { SettingsIndexPage } from '#/domain/settings/feature/settings-index-page'
import { ANTHROPIC_PROVIDER } from '#/domain/settings/test-fixtures'
import { clearAuth, setAuth } from '#/store/auth.store'

function json(body: unknown) {
	return new Response(JSON.stringify(body), {
		status: 200,
		headers: { 'Content-Type': 'application/json' },
	})
}

function route(body: Record<string, unknown>) {
	return async (input: unknown) => {
		const path = new URL(String(input)).pathname
		if (path in body) return json(body[path])
		return new Response('{}', { status: 500 })
	}
}

describe('SettingsIndexPage', () => {
	const fetchMock = vi.fn()

	beforeEach(() => {
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
	})

	async function show() {
		const client = new QueryClient({
			defaultOptions: { queries: { retry: false } },
		})
		await renderAtPath(
			<QueryClientProvider client={client}>
				<SettingsIndexPage />
			</QueryClientProvider>,
			'/settings',
		)
	}

	it('shows the live state of each card with its link', async () => {
		fetchMock.mockImplementation(
			route({
				'/api/v1/providers': { providers: [ANTHROPIC_PROVIDER] },
				'/api/v1/model-assignments': { assignments: null },
				'/api/v1/integrations/github': {
					installation: {
						account_login: 'ada',
						connected_at: '2026-10-16T20:05:19Z',
						connected_by: 'ada',
						installation_id: 1,
					},
				},
			}),
		)
		await show()

		expect(await screen.findByText('1 fournisseur')).toBeTruthy()
		expect(await screen.findByText('Modèles non assignés')).toBeTruthy()
		expect(await screen.findByText('GitHub connecté')).toBeTruthy()
		expect(
			screen.getByRole('link', { name: /Fournisseurs/ }).getAttribute('href'),
		).toBe('/settings/providers')
		expect(
			screen.getByRole('link', { name: /Modèles/ }).getAttribute('href'),
		).toBe('/settings/models')
		expect(
			screen.getByRole('link', { name: /Intégrations/ }).getAttribute('href'),
		).toBe('/settings/integrations')
	})

	it('says so when a state cannot be read', async () => {
		fetchMock.mockImplementation(route({}))
		await show()

		expect(await screen.findAllByText('Indisponible')).toHaveLength(3)
	})
})
