import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { KanbanPage } from '#/domain/kanban/feature/kanban-page'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

vi.mock('@tanstack/react-router', () => ({
	Link: ({ to, children }: { to: string; children: ReactNode }) => (
		<a href={to}>{children}</a>
	),
}))

const CARD: Schemas.CardResponse = {
	id: 'c1',
	title: 'Corriger le bug',
	description: 'Le détail complet de la carte',
	column: 'todo',
	provenance: {
		transcript_id: 't1',
		start: 0,
		end: 10,
		quote: 'il faut corriger',
	},
	created_by_run: 'r0',
	created_by_subject: 'ada',
	created_by_agent: 'capitainerie-agent-writer',
	version: 4,
	created_at: '2026-10-16T09:30:00Z',
	updated_at: '2026-10-16T09:30:00Z',
	acceptance_criteria: [],
	priority: 'normal',
	labels: [],
}

const ORIGIN_RUN: Schemas.RunResponse = {
	id: 'r0',
	status: 'succeeded',
	transcript_id: 't1',
	transcript_title: 'Point hebdo',
	title: 'Point hebdo',
	card_id: null,
	repository: null,
	initiated_by: 'ada',
	pipeline: ['extract', 'deduplicate', 'write'],
	failure: null,
	created_at: '2026-10-16T09:00:00Z',
	started_at: '2026-10-16T09:00:01Z',
	finished_at: '2026-10-16T09:01:00Z',
	trace_id: null,
}

const COMMENT: Schemas.CommentResponse = {
	id: 'm1',
	body: 'Mentionné à nouveau en réunion',
	author_subject: 'agent-user',
	author_actor: 'capitainerie-agent-deduplicator',
	created_at: '2026-10-16T10:00:00Z',
}

const DETAIL: Schemas.CardDetailResponse = {
	card: CARD,
	comments: [COMMENT],
	runs: [ORIGIN_RUN],
}

type Route = (init: RequestInit) => Response

describe('KanbanPage card panel', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function serve(table: Record<string, Route>) {
		fetchMock.mockImplementation((url: URL, init: RequestInit) => {
			const route = table[`${init.method} ${url.pathname}`]
			if (!route) throw new Error(`unexpected ${init.method} ${url.pathname}`)
			return Promise.resolve(route(init))
		})
	}

	function base(extra: Record<string, Route> = {}) {
		return {
			'GET /api/v1/cards': () => json(200, { items: [CARD] }),
			'GET /api/v1/cards/c1': () => json(200, DETAIL),
			'GET /api/v1/me': () =>
				json(200, {
					subject: 'ada',
					organization: { id: 'o1', slug: 'acme', name: 'Acme' },
				}),
			...extra,
		}
	}

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it('stays closed without a card in the URL', async () => {
		serve(base())
		render(<KanbanPage openCardId={undefined} onOpenCard={vi.fn()} />, {
			wrapper: harness.wrapper,
		})

		await screen.findByText('Corriger le bug')

		expect(screen.queryByRole('dialog')).toBeNull()
		expect(
			fetchMock.mock.calls.some(
				([url]) => (url as URL).pathname === '/api/v1/cards/c1',
			),
		).toBe(false)
	})

	it('opens the panel for the card of the URL with its description, runs and comments', async () => {
		serve(base())
		render(<KanbanPage openCardId="c1" onOpenCard={vi.fn()} />, {
			wrapper: harness.wrapper,
		})

		const panel = await screen.findByRole('dialog')

		expect(
			await within(panel).findByText('Le détail complet de la carte'),
		).toBeTruthy()
		expect(within(panel).getByText('« il faut corriger »')).toBeTruthy()
		expect(
			within(panel).getByRole('link', { name: 'Voir le run d’origine' }),
		).toBeTruthy()
		expect(
			await within(panel).findByText('Mentionné à nouveau en réunion'),
		).toBeTruthy()
		expect(within(panel).getByText('deduplicator')).toBeTruthy()
		expect(
			within(panel).getByRole('link', { name: 'Run d’origine' }),
		).toBeTruthy()
		expect(
			within(panel).getByRole('button', { name: 'Confier à Claude Code' }),
		).toBeTruthy()
	})

	it('removes the card from the URL on Escape', async () => {
		serve(base())
		const onOpenCard = vi.fn()
		const user = userEvent.setup()
		render(<KanbanPage openCardId="c1" onOpenCard={onOpenCard} />, {
			wrapper: harness.wrapper,
		})
		await screen.findByRole('dialog')

		await user.keyboard('{Escape}')

		expect(onOpenCard).toHaveBeenCalledWith(undefined)
	})

	it('puts the card in the URL when it is clicked', async () => {
		serve(base())
		const onOpenCard = vi.fn()
		const user = userEvent.setup()
		render(<KanbanPage openCardId={undefined} onOpenCard={onOpenCard} />, {
			wrapper: harness.wrapper,
		})

		await user.click(await screen.findByText('Corriger le bug'))

		expect(onOpenCard).toHaveBeenCalledWith('c1')
	})

	it('says so when the card is not in the organization', async () => {
		serve(
			base({
				'GET /api/v1/cards/zz': () =>
					json(404, { code: 'not_found', status: 404, message: 'not found' }),
			}),
		)
		render(<KanbanPage openCardId="zz" onOpenCard={vi.fn()} />, {
			wrapper: harness.wrapper,
		})

		expect(
			await screen.findByText(
				'Cette carte n’existe pas dans votre organisation.',
			),
		).toBeTruthy()
	})

	it('posts a comment and clears the field', async () => {
		const bodies: Array<unknown> = []
		serve(
			base({
				'POST /api/v1/cards/c1/comments': (init) => {
					bodies.push(JSON.parse(String(init.body)))
					return json(201, {
						id: 'm2',
						body: 'À traiter demain',
						author_subject: 'ada',
						author_actor: null,
						created_at: '2026-10-16T11:00:00Z',
					})
				},
			}),
		)
		const user = userEvent.setup()
		render(<KanbanPage openCardId="c1" onOpenCard={vi.fn()} />, {
			wrapper: harness.wrapper,
		})
		const panel = await screen.findByRole('dialog')
		const field = await within(panel).findByLabelText('Ajouter un commentaire')
		const send = within(panel).getByRole('button', { name: 'Commenter' })
		expect((send as HTMLButtonElement).disabled).toBe(true)

		await user.type(field, 'À traiter demain')
		await user.click(send)

		await waitFor(() => expect(bodies).toEqual([{ body: 'À traiter demain' }]))
		await waitFor(() => expect((field as HTMLTextAreaElement).value).toBe(''))
	})

	it('shows the server refusal of a comment', async () => {
		serve(
			base({
				'POST /api/v1/cards/c1/comments': () =>
					json(422, {
						code: 'invalid',
						status: 422,
						message: 'the comment is empty',
					}),
			}),
		)
		const user = userEvent.setup()
		render(<KanbanPage openCardId="c1" onOpenCard={vi.fn()} />, {
			wrapper: harness.wrapper,
		})
		const panel = await screen.findByRole('dialog')
		const field = await within(panel).findByLabelText('Ajouter un commentaire')

		await user.type(field, 'x')
		await user.click(within(panel).getByRole('button', { name: 'Commenter' }))

		expect(
			await within(panel).findByText(
				'Le commentaire est vide ou dépasse 10 000 caractères.',
			),
		).toBeTruthy()
	})

	it('moves the card from the panel with its version', async () => {
		const bodies: Array<unknown> = []
		serve(
			base({
				'POST /api/v1/cards/c1/move': (init) => {
					bodies.push(JSON.parse(String(init.body)))
					return json(200, { ...CARD, column: 'done', version: 5 })
				},
			}),
		)
		const user = userEvent.setup()
		render(<KanbanPage openCardId="c1" onOpenCard={vi.fn()} />, {
			wrapper: harness.wrapper,
		})
		const panel = await screen.findByRole('dialog')

		await user.selectOptions(
			await within(panel).findByLabelText('Déplacer vers…'),
			'done',
		)

		await waitFor(() =>
			expect(bodies).toEqual([{ to: 'done', expected_version: 4 }]),
		)
	})

	it('saves each field on its own through PATCH with the expected version', async () => {
		const bodies: Array<unknown> = []
		serve(
			base({
				'GET /api/v1/members': () =>
					json(200, {
						items: [
							{
								subject: 'u1',
								name: 'Claire Martin',
								email: null,
								last_seen_at: '2026-10-19T08:00:00Z',
							},
						],
					}),
				'PATCH /api/v1/cards/c1': (init) => {
					bodies.push(JSON.parse(String(init.body)))
					return json(200, { ...CARD, version: 5 })
				},
			}),
		)
		const user = userEvent.setup()
		render(<KanbanPage openCardId="c1" onOpenCard={vi.fn()} />, {
			wrapper: harness.wrapper,
		})
		const panel = await screen.findByRole('dialog')

		await user.selectOptions(
			await within(panel).findByLabelText('Priorité'),
			'urgent',
		)
		await waitFor(() => expect(bodies).toHaveLength(1))
		await waitFor(() =>
			expect(
				(within(panel).getByLabelText('Priorité') as HTMLSelectElement)
					.disabled,
			).toBe(false),
		)
		await user.click(
			within(panel).getByRole('button', { name: 'Modifier le titre' }),
		)
		const title = within(panel).getByLabelText('Titre')
		await user.clear(title)
		await user.type(title, 'Corriger le gros bug')
		await user.click(within(panel).getByRole('button', { name: 'Enregistrer' }))
		await waitFor(() => expect(bodies).toHaveLength(2))
		const responsable = await within(panel).findByRole('option', {
			name: 'Claire Martin',
		})
		expect(responsable).toBeTruthy()

		expect(bodies[0]).toEqual({ expected_version: 4, priority: 'urgent' })
		expect(bodies[1]).toEqual({
			expected_version: 4,
			title: 'Corriger le gros bug',
		})
	})

	it('reloads the card and says so when a field save meets a stale version', async () => {
		let boards = 0
		serve(
			base({
				'GET /api/v1/cards': () => {
					boards += 1
					return json(200, { items: [CARD] })
				},
				'PATCH /api/v1/cards/c1': () =>
					json(409, { code: 'conflict', status: 409, message: 'stale' }),
			}),
		)
		const user = userEvent.setup()
		render(<KanbanPage openCardId="c1" onOpenCard={vi.fn()} />, {
			wrapper: harness.wrapper,
		})
		const panel = await screen.findByRole('dialog')
		await within(panel).findByText('Le détail complet de la carte')
		const before = boards

		await user.selectOptions(within(panel).getByLabelText('Priorité'), 'high')

		expect(
			await within(panel).findByText('La carte a changé, rechargée.'),
		).toBeTruthy()
		await waitFor(() => expect(boards).toBeGreaterThan(before))
	})
})
