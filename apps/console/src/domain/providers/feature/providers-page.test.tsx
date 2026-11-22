import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { renderAtPath } from '#/components/layout/test-router'
import { ProvidersPage } from '#/domain/providers/feature/providers-page'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'

const SECRET = 'sk-ant-super-secret-1234'

function provider(
	kind: Schemas.ProviderKindDto,
	extra: Partial<Schemas.UserProviderResponse> = {},
): Schemas.UserProviderResponse {
	return {
		kind,
		base_url: null,
		default_model: 'claude-sonnet-5-5',
		preview: 'abcd',
		is_default: false,
		updated_at: '2026-11-01T00:00:00Z',
		...extra,
	}
}

const ANTHROPIC = provider('anthropic', { is_default: true })
const MISTRAL = provider('mistral', {
	default_model: 'mistral-large-latest',
	preview: 'wxyz',
})

async function open(routes: Parameters<typeof mockApi>[0]) {
	const fetchMock = mockApi(routes)
	await renderAtPath(withQueryClient(<ProvidersPage />), '/providers')
	return fetchMock
}

function calls(fetchMock: ReturnType<typeof mockApi>, method: string) {
	return fetchMock.mock.calls
		.filter(([, init]) => init?.method === method)
		.map(([url, init]) => ({
			path: new URL(String(url)).pathname,
			body: init?.body ? JSON.parse(String(init.body)) : undefined,
		}))
}

const card = (name: string) => screen.findByRole('article', { name })

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

describe('ProvidersPage', () => {
	it('lists connected providers with masked preview, default badge and model', async () => {
		await open({ 'GET /api/v1/me/providers': [ANTHROPIC, MISTRAL] })
		const anthropic = await card('Anthropic')
		expect(within(anthropic).getByText('Connecté')).toBeTruthy()
		expect(within(anthropic).getByText('••••abcd')).toBeTruthy()
		expect(within(anthropic).getByText('Par défaut')).toBeTruthy()
		expect(within(anthropic).getByText('claude-sonnet-5-5')).toBeTruthy()
		const mistral = await card('Mistral')
		expect(within(mistral).getByText('••••wxyz')).toBeTruthy()
		expect(within(mistral).queryByText('Par défaut')).toBeNull()
		const openai = await card('Compatible OpenAI')
		expect(within(openai).getByText('Non connecté')).toBeTruthy()
		expect(
			within(openai).getByRole('button', { name: 'Connecter' }),
		).toBeTruthy()
	})

	it('explains the empty state', async () => {
		await open({ 'GET /api/v1/me/providers': [] })
		expect(await screen.findByText('Aucun fournisseur connecté')).toBeTruthy()
		expect(screen.getByText(/s'exécutent avec votre propre/)).toBeTruthy()
		expect(screen.getAllByRole('button', { name: 'Connecter' })).toHaveLength(4)
	})

	it('shows a load error', async () => {
		await open({
			'GET /api/v1/me/providers': {
				status: 500,
				body: { code: 'boom', message: 'boom', status: 500 },
			},
		})
		expect(
			await screen.findByText('Impossible de charger vos fournisseurs.'),
		).toBeTruthy()
	})

	it('connects a provider and clears the key', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			'GET /api/v1/me/providers': [],
			'PUT /api/v1/me/providers/anthropic': provider('anthropic'),
		})
		const anthropic = await card('Anthropic')
		await user.click(
			within(anthropic).getByRole('button', { name: 'Connecter' }),
		)
		const key = screen.getByLabelText('Clé API') as HTMLInputElement
		expect(key.type).toBe('password')
		expect(key.value).toBe('')
		const model = screen.getByLabelText('Modèle par défaut')
		expect(model.getAttribute('placeholder')).toBe('claude-sonnet-5-5')
		await user.type(key, SECRET)
		await user.type(model, 'claude-sonnet-5-5')
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(await screen.findByText('Anthropic enregistré')).toBeTruthy()
		expect(calls(fetchMock, 'PUT')[0]).toEqual({
			path: '/api/v1/me/providers/anthropic',
			body: {
				api_key: SECRET,
				default_model: 'claude-sonnet-5-5',
				is_default: true,
			},
		})
		expect(screen.queryByLabelText('Clé API')).toBeNull()
		await user.click(
			within(await card('Anthropic')).getByRole('button', {
				name: 'Connecter',
			}),
		)
		expect((screen.getByLabelText('Clé API') as HTMLInputElement).value).toBe(
			'',
		)
		expect(document.body.innerHTML).not.toContain(SECRET)
	})

	it('requires a base url for the OpenAI-compatible kind', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			'GET /api/v1/me/providers': [],
			'PUT /api/v1/me/providers/openai_compatible':
				provider('openai_compatible'),
		})
		await user.click(
			within(await card('Compatible OpenAI')).getByRole('button', {
				name: 'Connecter',
			}),
		)
		expect(
			screen.getByText(/ex\. Gemini : https:\/\/generativelanguage/),
		).toBeTruthy()
		const model = screen.getByLabelText('Modèle par défaut')
		expect(model.getAttribute('placeholder')).toBe('gemini-3-flash-preview')
		await user.type(model, 'gemini-3-flash-preview')
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(await screen.findByText(/L'URL de base doit commencer/)).toBeTruthy()
		expect(calls(fetchMock, 'PUT')).toHaveLength(0)
		await user.type(
			screen.getByLabelText(/^URL de base/),
			'https://generativelanguage.googleapis.com/v1beta/openai',
		)
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		await screen.findByText('Compatible OpenAI enregistré')
		expect(calls(fetchMock, 'PUT')[0]?.body).toEqual({
			api_key: '',
			base_url: 'https://generativelanguage.googleapis.com/v1beta/openai',
			default_model: 'gemini-3-flash-preview',
			is_default: true,
		})
	})

	it('reports a 422 in French and keeps the dialog open', async () => {
		const user = userEvent.setup()
		await open({
			'GET /api/v1/me/providers': [],
			'PUT /api/v1/me/providers/mistral': {
				status: 422,
				body: { code: 'invalid', message: 'x', status: 422 },
			},
		})
		await user.click(
			within(await card('Mistral')).getByRole('button', { name: 'Connecter' }),
		)
		await user.type(screen.getByLabelText('Clé API'), 'bad')
		await user.type(screen.getByLabelText('Modèle par défaut'), 'm')
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(await screen.findByText(/Paramètres refusés/)).toBeTruthy()
		expect(screen.getByLabelText('Clé API')).toBeTruthy()
	})

	it('tests a provider: success then failure', async () => {
		const user = userEvent.setup()
		let ok = true
		const fetchMock = await open({
			'GET /api/v1/me/providers': [ANTHROPIC],
			'POST /api/v1/me/providers/anthropic/test': { ok: true, error: null },
		})
		const inner = fetchMock.getMockImplementation()
		fetchMock.mockImplementation(async (input, init) => {
			if (
				new URL(String(input)).pathname ===
				'/api/v1/me/providers/anthropic/test'
			) {
				return new Response(
					JSON.stringify(
						ok ? { ok: true } : { ok: false, error: 'clé rejetée' },
					),
					{ status: 200, headers: { 'Content-Type': 'application/json' } },
				)
			}
			return (inner as NonNullable<typeof inner>)(input, init)
		})
		const anthropic = await card('Anthropic')
		await user.click(within(anthropic).getByRole('button', { name: 'Tester' }))
		expect(await screen.findByText(/Connexion réussie/)).toBeTruthy()
		ok = false
		await user.click(within(anthropic).getByRole('button', { name: 'Tester' }))
		expect(await screen.findByText('clé rejetée')).toBeTruthy()
	})

	it('maps a 404 on test to a French message', async () => {
		const user = userEvent.setup()
		await open({
			'GET /api/v1/me/providers': [ANTHROPIC],
			'POST /api/v1/me/providers/anthropic/test': {
				status: 404,
				body: { code: 'not_found', message: 'x', status: 404 },
			},
		})
		await user.click(
			within(await card('Anthropic')).getByRole('button', { name: 'Tester' }),
		)
		expect(
			await screen.findByText("Ce fournisseur n'est pas connecté."),
		).toBeTruthy()
	})

	it('deletes after confirmation', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			'GET /api/v1/me/providers': [ANTHROPIC],
			'DELETE /api/v1/me/providers/anthropic': {
				status: 204,
				body: undefined,
			},
		})
		await user.click(
			within(await card('Anthropic')).getByRole('button', {
				name: 'Supprimer',
			}),
		)
		expect(calls(fetchMock, 'DELETE')).toHaveLength(0)
		await user.click(
			within(screen.getByRole('alertdialog')).getByRole('button', {
				name: 'Supprimer',
			}),
		)
		expect(await screen.findByText('Anthropic supprimé')).toBeTruthy()
		expect(calls(fetchMock, 'DELETE')).toHaveLength(1)
	})

	it('sets another provider as default without resending a key', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			'GET /api/v1/me/providers': [ANTHROPIC, MISTRAL],
			'PUT /api/v1/me/providers/mistral': provider('mistral', {
				is_default: true,
			}),
		})
		const mistral = await card('Mistral')
		await user.click(
			within(mistral).getByRole('button', { name: 'Définir par défaut' }),
		)
		await waitFor(() => expect(calls(fetchMock, 'PUT')).toHaveLength(1))
		expect(calls(fetchMock, 'PUT')[0]?.body).toEqual({
			default_model: 'mistral-large-latest',
			is_default: true,
		})
	})

	it('never prefills the key when editing', async () => {
		const user = userEvent.setup()
		await open({ 'GET /api/v1/me/providers': [ANTHROPIC] })
		await user.click(
			within(await card('Anthropic')).getByRole('button', { name: 'Modifier' }),
		)
		expect((screen.getByLabelText('Clé API') as HTMLInputElement).value).toBe(
			'',
		)
		expect(
			(screen.getByLabelText('Modèle par défaut') as HTMLInputElement).value,
		).toBe('claude-sonnet-5-5')
	})
})
