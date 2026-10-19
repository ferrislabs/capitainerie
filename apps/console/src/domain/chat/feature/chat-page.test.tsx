import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { ChatPage } from '#/domain/chat/feature/chat-page'
import { openSse, sseEvent } from '#/domain/chat/test-utils'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

const navigate = vi.fn()

vi.mock('@tanstack/react-router', () => ({
	Link: ({
		to,
		children,
		search,
	}: {
		to: string
		children: ReactNode
		search?: { card?: string }
	}) => (
		<a href={search?.card ? `${to}?card=${search.card}` : to}>{children}</a>
	),
	useNavigate: () => navigate,
}))

const NOW = '2026-10-19T10:00:00Z'

function summary(
	overrides: Partial<Schemas.ConversationSummaryResponse> = {},
): Schemas.ConversationSummaryResponse {
	return {
		id: 'c1',
		title: 'Ajoute un README au dépôt test',
		model: 'claude-sonnet-5-5',
		input_tokens: 1200,
		output_tokens: 300,
		cost_micros: 0,
		full: false,
		archived: false,
		created_at: NOW,
		updated_at: NOW,
		...overrides,
	}
}

function message(
	overrides: Partial<Schemas.MessageResponse> & { id: string; seq: number },
): Schemas.MessageResponse {
	return {
		role: 'user',
		text: '',
		tool_calls: [],
		cards: [],
		status: 'complete',
		created_at: NOW,
		...overrides,
	}
}

function card(column: Schemas.ColumnDto, version = 1): Schemas.CardResponse {
	return {
		id: 'k1',
		title: 'Ajouter un README',
		description: 'Pour Acme',
		column,
		created_by_subject: 'ada',
		created_by_agent: 'capitainerie-agent-captain',
		version,
		created_at: NOW,
		updated_at: NOW,
	}
}

type Route = (init: RequestInit) => Response | Promise<Response>

describe('ChatPage', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function serve(table: Record<string, Route>) {
		fetchMock.mockImplementation((url: URL | string, init: RequestInit) => {
			const path =
				typeof url === 'string' ? new URL(url).pathname : url.pathname
			const route = table[`${init.method} ${path}`]
			if (!route) throw new Error(`unexpected ${init.method} ${path}`)
			return Promise.resolve(route(init))
		})
	}

	function base(
		conversation: {
			summary?: Partial<Schemas.ConversationSummaryResponse>
			messages: Array<Schemas.MessageResponse>
		},
		extra: Record<string, Route> = {},
		cards: Array<Schemas.CardResponse> = [],
	) {
		return {
			'GET /api/v1/conversations/c1': () =>
				json(200, {
					conversation: summary(conversation.summary),
					messages: conversation.messages,
				}),
			'GET /api/v1/cards': () => json(200, { items: cards }),
			'GET /api/v1/model-assignments': () =>
				json(200, {
					assignments: {
						extractor: { provider: 'anthropic', model: 'claude-haiku-5-5' },
						judge: { provider: 'anthropic', model: 'claude-haiku-5-5' },
						writer: { provider: 'anthropic', model: 'claude-sonnet-5-5' },
						embeddings: { provider: 'mistral', model: 'mistral-embed' },
					},
				}),
			...extra,
		}
	}

	function renderPage() {
		return render(<ChatPage conversationId="c1" />, {
			wrapper: harness.wrapper,
		})
	}

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		navigate.mockReset()
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it('shows a skeleton, then the title, the usage and the conversation', async () => {
		serve(
			base({
				messages: [
					message({ id: 'm1', seq: 1, text: 'Bonjour' }),
					message({ id: 'm2', seq: 2, role: 'assistant', text: 'Salut !' }),
				],
			}),
		)

		renderPage()

		expect(screen.getByText('Chargement…')).toBeTruthy()
		expect(
			await screen.findByRole('heading', {
				name: 'Ajoute un README au dépôt test',
			}),
		).toBeTruthy()
		expect(screen.getByText(/claude-sonnet-5-5 · 1\s?500 tokens/)).toBeTruthy()
		expect(screen.getByText('Bonjour')).toBeTruthy()
		expect(screen.getByText('Salut !')).toBeTruthy()
		expect(screen.getByRole('link', { name: /Conversations/ })).toBeTruthy()
		expect(
			fetchMock.mock.calls.some(([, init]) => init?.method === 'POST'),
		).toBe(false)
	})

	it('says the conversation cannot be found', async () => {
		serve({
			'GET /api/v1/conversations/c1': () =>
				json(404, { code: 'not_found', status: 404, message: 'not found' }),
			'GET /api/v1/cards': () => json(200, { items: [] }),
			'GET /api/v1/model-assignments': () => json(200, { assignments: null }),
		})

		renderPage()

		expect((await screen.findByRole('alert')).textContent).toContain(
			'Cette conversation est introuvable.',
		)
	})

	it('starts the first turn by itself, streams text and collapsed tool calls, then the stored answer', async () => {
		let feed: ReturnType<typeof openSse> | undefined
		let answered = false
		const user = message({ id: 'm1', seq: 1, text: 'Ajoute un README' })
		serve(
			base(
				{ messages: [user] },
				{
					'GET /api/v1/conversations/c1': () =>
						json(200, {
							conversation: summary(),
							messages: answered
								? [
										user,
										message({
											id: 'm2',
											seq: 2,
											role: 'assistant',
											text: 'Voilà, une carte est proposée.',
											tool_calls: [
												{
													name: 'search_cards',
													arguments: '{"query":"readme"}',
													succeeded: true,
													latency_ms: 12,
													summary: '3 cartes trouvées',
												},
											],
											cards: [{ card_id: 'k1', title: 'Ajouter un README' }],
										}),
									]
								: [user],
						}),
					'POST /api/v1/conversations/c1/messages': (init) => {
						feed = openSse(init.signal as AbortSignal)
						return feed.response
					},
				},
				[card('proposed')],
			),
		)

		renderPage()

		await waitFor(() => expect(feed).toBeDefined())
		expect(await screen.findByText('Le Capitaine réfléchit…')).toBeTruthy()
		const [, init] = fetchMock.mock.calls.find(
			([, call]) => call?.method === 'POST',
		) as [unknown, RequestInit]
		expect(JSON.parse(String(init.body))).toEqual({})

		feed?.push(sseEvent('delta', { text: 'Je regarde' }))
		expect(await screen.findByText('Je regarde')).toBeTruthy()
		feed?.push(
			sseEvent('tool', {
				phase: 'start',
				id: 't1',
				name: 'search_cards',
				arguments: '{"query":"readme"}',
			}),
		)
		expect(await screen.findByText('Cherche des cartes…')).toBeTruthy()
		feed?.push(
			sseEvent('tool', {
				phase: 'end',
				id: 't1',
				name: 'search_cards',
				succeeded: true,
				latency_ms: 12,
				summary: '3 cartes trouvées',
			}),
		)
		const headline = await screen.findByText('A cherché 3 cartes')
		expect(headline.closest('details')?.hasAttribute('open')).toBe(false)
		feed?.push(sseEvent('card', { card_id: 'k1', title: 'Ajouter un README' }))
		const proposal = await screen.findByRole('article', {
			name: 'Carte proposée : Ajouter un README',
		})
		expect(
			within(proposal).getByRole('button', { name: 'Valider' }),
		).toBeTruthy()

		answered = true
		feed?.push(
			sseEvent('done', {
				status: 'complete',
				usage: { input_tokens: 1, output_tokens: 1 },
			}),
		)
		feed?.close()

		expect(
			await screen.findByText('Voilà, une carte est proposée.'),
		).toBeTruthy()
		await waitFor(() => expect(screen.queryByText('Je regarde')).toBeNull())
		expect(screen.getAllByRole('article')).toHaveLength(1)
		expect(
			fetchMock.mock.calls.filter(([, call]) => call?.method === 'POST'),
		).toHaveLength(1)
	})

	it('sends the typed message, shows it at once and clears the field', async () => {
		let feed: ReturnType<typeof openSse> | undefined
		serve(
			base(
				{
					messages: [
						message({ id: 'm1', seq: 1, text: 'Bonjour' }),
						message({ id: 'm2', seq: 2, role: 'assistant', text: 'Salut !' }),
					],
				},
				{
					'POST /api/v1/conversations/c1/messages': (init) => {
						feed = openSse(init.signal as AbortSignal)
						return feed.response
					},
				},
			),
		)
		renderPage()
		const field = await screen.findByLabelText('Message au Capitaine')

		await userEvent.type(field, 'Plutôt dans le dépôt api{Enter}')

		await waitFor(() => expect(feed).toBeDefined())
		expect(
			screen.getByText('Plutôt dans le dépôt api', { selector: 'p' }),
		).toBeTruthy()
		expect((field as HTMLTextAreaElement).value).toBe('')
		const [, init] = fetchMock.mock.calls.find(
			([, call]) => call?.method === 'POST',
		) as [unknown, RequestInit]
		expect(JSON.parse(String(init.body))).toEqual({
			message: 'Plutôt dans le dépôt api',
		})
		expect(screen.getByRole('button', { name: 'Interrompre' })).toBeTruthy()
		feed?.close()
	})

	it('aborts the turn on Escape, anywhere on the page', async () => {
		let signal: AbortSignal | undefined
		let feed: ReturnType<typeof openSse> | undefined
		serve(
			base(
				{ messages: [message({ id: 'm1', seq: 1, text: 'Bonjour' })] },
				{
					'POST /api/v1/conversations/c1/messages': (init) => {
						signal = init.signal as AbortSignal
						feed = openSse(signal)
						return feed.response
					},
				},
			),
		)
		renderPage()
		await waitFor(() => expect(feed).toBeDefined())
		feed?.push(sseEvent('delta', { text: 'Je cher' }))
		await screen.findByText('Je cher')

		await userEvent.keyboard('{Escape}')

		await waitFor(() => expect(signal?.aborted).toBe(true))
		await waitFor(() =>
			expect(screen.queryByRole('button', { name: 'Interrompre' })).toBeNull(),
		)
	})

	it('shows the error of a failed turn and does not retry by itself', async () => {
		serve(
			base(
				{ messages: [message({ id: 'm1', seq: 1, text: 'Bonjour' })] },
				{
					'POST /api/v1/conversations/c1/messages': () =>
						json(409, {
							code: 'captain_not_configured',
							status: 409,
							message: 'none',
						}),
				},
			),
		)

		renderPage()

		expect(
			await screen.findByText(/Aucun modèle ne sert le Capitaine/),
		).toBeTruthy()
		await new Promise((resolve) => setTimeout(resolve, 50))
		expect(
			fetchMock.mock.calls.filter(([, call]) => call?.method === 'POST'),
		).toHaveLength(1)
	})

	it('marks an interrupted and a failed answer with a word', async () => {
		serve(
			base({
				messages: [
					message({ id: 'm1', seq: 1, text: 'Bonjour' }),
					message({
						id: 'm2',
						seq: 2,
						role: 'assistant',
						text: 'Je cher',
						status: 'interrupted',
					}),
					message({ id: 'm3', seq: 3, text: 'Encore' }),
					message({
						id: 'm4',
						seq: 4,
						role: 'assistant',
						text: '',
						status: 'failed',
					}),
				],
			}),
		)

		renderPage()

		expect(await screen.findByText('Réponse interrompue')).toBeTruthy()
		expect(screen.getByText('Le Capitaine n’a pas pu répondre')).toBeTruthy()
	})

	it('locks the composer and points to a new conversation when the limit is reached', async () => {
		serve(
			base({
				summary: { full: true },
				messages: [
					message({ id: 'm1', seq: 1, text: 'Bonjour' }),
					message({ id: 'm2', seq: 2, role: 'assistant', text: 'Salut' }),
				],
			}),
		)

		renderPage()

		expect(await screen.findByText(/a atteint sa limite/)).toBeTruthy()
		expect(
			(screen.getByLabelText('Message au Capitaine') as HTMLTextAreaElement)
				.disabled,
		).toBe(true)
		expect(
			screen
				.getByRole('link', { name: 'Nouvelle conversation' })
				.getAttribute('href'),
		).toBe('/')
	})

	describe('proposed cards', () => {
		const proposed = [
			message({ id: 'm1', seq: 1, text: 'Ajoute un README' }),
			message({
				id: 'm2',
				seq: 2,
				role: 'assistant',
				text: 'Proposé.',
				cards: [{ card_id: 'k1', title: 'Ajouter un README' }],
			}),
		]

		it('validates a card with its current version through the kanban endpoint', async () => {
			let moved: unknown
			serve(
				base(
					{ messages: proposed },
					{
						'POST /api/v1/cards/k1/move': (init) => {
							moved = JSON.parse(String(init.body))
							return json(200, card('todo', 3))
						},
					},
					[card('proposed', 2)],
				),
			)
			renderPage()

			await userEvent.click(
				await screen.findByRole('button', { name: 'Valider' }),
			)

			await waitFor(() =>
				expect(moved).toEqual({ to: 'todo', expected_version: 2 }),
			)
		})

		it('rejects a card', async () => {
			let moved: unknown
			serve(
				base(
					{ messages: proposed },
					{
						'POST /api/v1/cards/k1/move': (init) => {
							moved = JSON.parse(String(init.body))
							return json(200, card('rejected', 3))
						},
					},
					[card('proposed', 2)],
				),
			)
			renderPage()

			await userEvent.click(
				await screen.findByRole('button', { name: 'Rejeter' }),
			)

			await waitFor(() =>
				expect(moved).toEqual({ to: 'rejected', expected_version: 2 }),
			)
		})

		it('links to the card on the board and shows a settled card as a word', async () => {
			serve(base({ messages: proposed }, {}, [card('done', 5)]))
			renderPage()

			expect(await screen.findByText('Terminé')).toBeTruthy()
			expect(
				screen.getByRole('link', { name: 'Ouvrir' }).getAttribute('href'),
			).toBe('/kanban?card=k1')
			expect(screen.queryByRole('button', { name: 'Valider' })).toBeNull()
		})

		it('hands a validated card to Claude Code through the execution dialog', async () => {
			serve(
				base(
					{ messages: proposed },
					{
						'GET /api/v1/integrations/github': () =>
							json(200, {
								app: null,
								installation: null,
							}),
					},
					[card('todo', 3)],
				),
			)
			renderPage()

			await userEvent.click(
				await screen.findByRole('button', { name: 'Confier à Claude Code' }),
			)

			expect(await screen.findByRole('dialog')).toBeTruthy()
		})
	})
})
