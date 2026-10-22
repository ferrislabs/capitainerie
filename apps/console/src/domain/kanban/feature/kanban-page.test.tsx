import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { KanbanPage } from '#/domain/kanban/feature/kanban-page'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'
import { chooseValue } from '#/test-utils/combobox'

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

const ACME = { id: 'cl1', name: 'Acme' }

const ODOO_STAGING: Schemas.IntegrationInstanceResponse = {
	id: 'i2',
	client_id: 'cl1',
	kind: 'odoo_jsonrpc',
	name: 'Odoo recette',
	environment: 'staging',
	base_url: 'https://recette.acme.test',
	database: 'acme',
	login: 'api@acme.test',
	secret_preview: '…9f8e',
	server_version: '17.0',
	checked_at: null,
	created_by: 'ada',
	created_at: '2026-10-16T09:00:00Z',
	updated_at: '2026-10-16T09:00:00Z',
	archived_at: null,
}

const INTERVENTION_RUN: Schemas.RunResponse = {
	id: 'r5',
	status: 'succeeded',
	card_id: 'c1',
	client: ACME,
	title: null,
	repository: null,
	initiated_by: 'ada',
	pipeline: ['intervene'],
	failure: null,
	created_at: '2026-10-16T12:00:00Z',
	started_at: '2026-10-16T12:00:01Z',
	finished_at: '2026-10-16T12:01:00Z',
	trace_id: null,
}

const REFUSED_ACTION: Schemas.IntegrationActionResponse = {
	id: 'a1',
	run_id: 'r5',
	card_id: 'c1',
	instance_id: 'i2',
	instance_name: 'Odoo recette',
	instance_environment: 'staging',
	instance_kind: 'odoo_jsonrpc',
	actor: 'capitainerie-agent-integrator',
	tool: 'odoo_write',
	access: 'write',
	target: { model: 'res.partner', ids: [2] },
	arguments: { model: 'res.partner', ids: [2], values: { name: 'Acme' } },
	outcome: 'denied',
	reason_code: 'write_not_granted',
	revertible: false,
	created_at: '2026-10-16T12:00:30Z',
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

		await chooseValue(
			user,
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

		await chooseValue(
			user,
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
		const assignee = within(panel).getByLabelText('Responsable')
		await waitFor(() => expect(assignee).toHaveProperty('disabled', false))
		await user.click(assignee)
		expect(
			await screen.findByRole('option', { name: 'Claire Martin' }),
		).toBeTruthy()

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

		await chooseValue(user, within(panel).getByLabelText('Priorité'), 'high')

		expect(
			await within(panel).findByText('La carte a changé, rechargée.'),
		).toBeTruthy()
		await waitFor(() => expect(boards).toBeGreaterThan(before))
	})

	it('explains in the panel why a card without client cannot go to the Intervenant', async () => {
		serve(base())
		render(<KanbanPage openCardId="c1" onOpenCard={vi.fn()} />, {
			wrapper: harness.wrapper,
		})
		const panel = await screen.findByRole('dialog')

		const button = (await within(panel).findByRole('button', {
			name: 'Confier à l’Intervenant',
		})) as HTMLButtonElement

		expect(button.disabled).toBe(true)
		expect(
			within(panel).getByText('Rattachez d’abord la carte à un client'),
		).toBeTruthy()
	})

	it('hands a card to the Intervenant from the board and follows the run', async () => {
		const bodies: Array<unknown> = []
		const linked = { ...CARD, client: ACME }
		serve(
			base({
				'GET /api/v1/cards': () => json(200, { items: [linked] }),
				'GET /api/v1/clients/cl1/integrations': () =>
					json(200, { items: [ODOO_STAGING] }),
				'POST /api/v1/cards/c1/interventions': (init) => {
					bodies.push(JSON.parse(String(init.body)))
					return json(202, {
						card: { ...linked, column: 'in_progress', version: 5 },
						run_id: 'run-9',
					})
				},
			}),
		)
		const user = userEvent.setup()
		render(<KanbanPage openCardId={undefined} onOpenCard={vi.fn()} />, {
			wrapper: harness.wrapper,
		})

		const button = await screen.findByRole('button', {
			name: 'Confier à l’Intervenant',
		})
		await waitFor(() =>
			expect((button as HTMLButtonElement).disabled).toBe(false),
		)
		await user.click(button)
		const dialog = await screen.findByRole('dialog')
		await user.click(
			within(
				await within(dialog).findByRole('radiogroup', { name: 'Odoo recette' }),
			).getByRole('radio', { name: 'Lecture' }),
		)
		await user.click(within(dialog).getByRole('button', { name: 'Confier' }))

		expect(
			await screen.findByText(/Carte confiée à l’Intervenant/),
		).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'Suivre l’intervention' })
				.getAttribute('href'),
		).toBe('/runs/$runId')
		expect(bodies).toEqual([
			{ grants: [{ instance_id: 'i2', access: 'read' }], expected_version: 4 },
		])
		await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
	})

	it('reads the journal of the Intervenant on the card', async () => {
		serve(
			base({
				'GET /api/v1/cards/c1': () =>
					json(200, { ...DETAIL, runs: [ORIGIN_RUN, INTERVENTION_RUN] }),
				'GET /api/v1/cards/c1/integration-actions': () =>
					json(200, { items: [REFUSED_ACTION] }),
			}),
		)
		render(<KanbanPage openCardId="c1" onOpenCard={vi.fn()} />, {
			wrapper: harness.wrapper,
		})
		const panel = await screen.findByRole('dialog')

		expect(
			await within(panel).findByRole('heading', {
				name: 'Journal des intégrations',
			}),
		).toBeTruthy()
		expect(within(panel).getByText('Odoo · modifier')).toBeTruthy()
		expect(within(panel).getByText('Refusée')).toBeTruthy()
		expect(
			within(panel).getByText('écriture non autorisée sur cette instance'),
		).toBeTruthy()
		expect(
			within(panel).getByRole('link', { name: 'Intervention' }),
		).toBeTruthy()
	})

	it('has no journal without an intervention run', async () => {
		serve(base())
		render(<KanbanPage openCardId="c1" onOpenCard={vi.fn()} />, {
			wrapper: harness.wrapper,
		})
		const panel = await screen.findByRole('dialog')
		await within(panel).findByText('Mentionné à nouveau en réunion')

		expect(
			within(panel).queryByRole('heading', {
				name: 'Journal des intégrations',
			}),
		).toBeNull()
		expect(
			fetchMock.mock.calls.some(([url]) =>
				(url as URL).pathname.endsWith('/integration-actions'),
			),
		).toBe(false)
	})
})
