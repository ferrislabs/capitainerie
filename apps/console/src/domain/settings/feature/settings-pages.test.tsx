import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { ADMIN_ROLE } from '#/domain/identity/roles'
import { ModelsPage } from '#/domain/settings/feature/models-page'
import { ProvidersPage } from '#/domain/settings/feature/providers-page'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import {
	BGE,
	CLAUDE_LOOKALIKE,
	COMPATIBLE_PROVIDER,
	QWEN,
	SONNET,
	VOXTRAL,
	WHISPER,
} from '#/domain/settings/test-fixtures'
import { clearAuth, setAuth } from '#/store/auth.store'

vi.mock('@tanstack/react-router', () => ({
	Link: ({ to, children }: { to: string; children: React.ReactNode }) => (
		<a href={to}>{children}</a>
	),
}))

function tokenWithRoles(roles: Array<string>) {
	const payload = btoa(JSON.stringify({ realm_access: { roles } }))
	return `e30.${payload.replace(/=+$/, '')}.sig`
}

const SECRET = 'mistral-fake-key-9z9z'

type Route = (init: RequestInit) => Response

function routes(table: Record<string, Route>) {
	return (url: URL, init: RequestInit) => {
		const key = `${init.method} ${url.pathname}`
		const route = table[key]
		if (!route) throw new Error(`unexpected ${key}`)
		return Promise.resolve(route(init))
	}
}

describe('settings pages', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it('validates, saves a key, and clears it from the form', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		const bodies: Array<unknown> = []
		fetchMock.mockImplementation(
			routes({
				'GET /api/v1/providers': () =>
					json(200, { providers: [COMPATIBLE_PROVIDER] }),
				'PUT /api/v1/providers/mistral': (init) => {
					bodies.push(JSON.parse(String(init.body)))
					return json(422, {
						code: 'unprocessable',
						status: 422,
						message: 'invalid api key',
					})
				},
			}),
		)
		const user = userEvent.setup()
		render(<ProvidersPage />, { wrapper: harness.wrapper })

		await user.click(
			await screen.findByRole('button', { name: 'Configurer : Mistral' }),
		)
		const dialog = screen.getByRole('dialog', { name: 'Configurer Mistral' })
		await user.click(
			within(dialog).getByRole('button', { name: 'Enregistrer' }),
		)
		expect(
			await within(dialog).findByText('La clé d’API est obligatoire.'),
		).toBeTruthy()
		expect(bodies).toHaveLength(0)

		const key = within(dialog).getByLabelText('Clé d’API') as HTMLInputElement
		await user.click(key)
		await user.paste(SECRET)
		await user.click(
			within(dialog).getByRole('button', { name: 'Enregistrer' }),
		)

		await waitFor(() => expect(bodies).toHaveLength(1))
		expect(await within(dialog).findByText('invalid api key')).toBeTruthy()
		expect(bodies).toEqual([{ api_key: SECRET }])
		expect(key.value).toBe('')
		expect(document.body.innerHTML).not.toContain(SECRET)
	}, 15_000)

	it('shows members a read-only providers page', async () => {
		setAuth({ accessToken: tokenWithRoles(['member']) })
		fetchMock.mockImplementation(
			routes({
				'GET /api/v1/providers': () =>
					json(200, { providers: [COMPATIBLE_PROVIDER] }),
			}),
		)
		render(<ProvidersPage />, { wrapper: harness.wrapper })

		expect(await screen.findByText('Sans clé')).toBeTruthy()
		expect(
			screen.getByText(/Seuls les administrateurs de l’organisation/),
		).toBeTruthy()
		expect(screen.queryByRole('button')).toBeNull()
	})

	it('assigns the four roles from the worker default and saves', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		let saved: unknown = null
		fetchMock.mockImplementation(
			routes({
				'GET /api/v1/providers': () =>
					json(200, { providers: [COMPATIBLE_PROVIDER] }),
				'GET /api/v1/models': () =>
					json(200, { models: [QWEN, BGE], errors: [] }),
				'GET /api/v1/model-assignments': () =>
					json(200, { assignments: saved }),
				'PUT /api/v1/model-assignments': (init) => {
					saved = JSON.parse(String(init.body))
					return json(200, { assignments: saved })
				},
			}),
		)
		const user = userEvent.setup()
		render(<ModelsPage />, { wrapper: harness.wrapper })

		expect(
			await screen.findByText(/Configuration par défaut du worker/),
		).toBeTruthy()
		const save = screen.getByRole('button', { name: 'Enregistrer' })
		for (const label of ['Extracteur', 'Juge de doublons', 'Rédacteur']) {
			await user.selectOptions(
				screen.getByLabelText(label),
				'openai_compatible:qwen2.5:7b-instruct',
			)
		}
		expect((save as HTMLButtonElement).disabled).toBe(true)
		await user.selectOptions(
			screen.getByLabelText('Embeddings'),
			'openai_compatible:bge-m3:latest',
		)
		await user.click(save)

		expect(await screen.findByText(/Modèles enregistrés/)).toBeTruthy()
		expect(saved).toEqual({
			extractor: { provider: 'openai_compatible', model: QWEN.id },
			judge: { provider: 'openai_compatible', model: QWEN.id },
			writer: { provider: 'openai_compatible', model: QWEN.id },
			embeddings: { provider: 'openai_compatible', model: BGE.id },
			transcriber: null,
		})
		await waitFor(() =>
			expect(
				screen.queryByText(/Configuration par défaut du worker/),
			).toBeNull(),
		)
		expect(
			screen.getByText('qwen2.5:7b-instruct', { selector: 'span' }),
		).toBeTruthy()
	})

	it('saves the executor along with a saved set of roles', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		const ref = { provider: 'openai_compatible', model: QWEN.id }
		let saved: unknown = {
			extractor: ref,
			judge: ref,
			writer: ref,
			embeddings: { provider: 'openai_compatible', model: BGE.id },
		}
		fetchMock.mockImplementation(
			routes({
				'GET /api/v1/providers': () =>
					json(200, { providers: [COMPATIBLE_PROVIDER] }),
				'GET /api/v1/models': () =>
					json(200, {
						models: [QWEN, BGE, SONNET, CLAUDE_LOOKALIKE],
						errors: [],
					}),
				'GET /api/v1/model-assignments': () =>
					json(200, { assignments: saved }),
				'PUT /api/v1/model-assignments': (init) => {
					saved = JSON.parse(String(init.body))
					return json(200, { assignments: saved })
				},
			}),
		)
		const user = userEvent.setup()
		render(<ModelsPage />, { wrapper: harness.wrapper })

		const executor = await screen.findByLabelText('Exécutant')
		expect(
			(screen.getByRole('button', { name: 'Enregistrer' }) as HTMLButtonElement)
				.disabled,
		).toBe(true)
		await user.selectOptions(executor, 'claude-sonnet-5')
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))

		expect(await screen.findByText(/Modèles enregistrés/)).toBeTruthy()
		expect(saved).toMatchObject({ executor: 'claude-sonnet-5' })
	})

	it('assigns, then clears, the optional transcriber', async () => {
		setAuth({ accessToken: tokenWithRoles([ADMIN_ROLE]) })
		const ref = { provider: 'openai_compatible', model: QWEN.id }
		let saved: unknown = {
			extractor: ref,
			judge: ref,
			writer: ref,
			embeddings: { provider: 'openai_compatible', model: BGE.id },
		}
		const puts: Array<unknown> = []
		fetchMock.mockImplementation(
			routes({
				'GET /api/v1/providers': () =>
					json(200, { providers: [COMPATIBLE_PROVIDER] }),
				'GET /api/v1/models': () =>
					json(200, {
						models: [QWEN, BGE, SONNET, VOXTRAL, WHISPER],
						errors: [],
					}),
				'GET /api/v1/model-assignments': () =>
					json(200, { assignments: saved }),
				'PUT /api/v1/model-assignments': (init) => {
					saved = JSON.parse(String(init.body))
					puts.push(saved)
					return json(200, { assignments: saved })
				},
			}),
		)
		const user = userEvent.setup()
		render(<ModelsPage />, { wrapper: harness.wrapper })

		const transcriber = (await screen.findByLabelText(
			'Transcripteur',
		)) as HTMLSelectElement
		expect(transcriber.value).toBe('')
		await user.selectOptions(transcriber, 'mistral:voxtral-mini-latest')
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))

		await waitFor(() => expect(puts).toHaveLength(1))
		expect(puts[0]).toMatchObject({
			transcriber: { provider: 'mistral', model: VOXTRAL.id },
		})
		await waitFor(() =>
			expect(
				(screen.getByLabelText('Transcripteur') as HTMLSelectElement).value,
			).toBe('mistral:voxtral-mini-latest'),
		)

		await user.selectOptions(screen.getByLabelText('Transcripteur'), '')
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))

		await waitFor(() => expect(puts).toHaveLength(2))
		expect(puts[1]).toMatchObject({ transcriber: null })
	})
})
