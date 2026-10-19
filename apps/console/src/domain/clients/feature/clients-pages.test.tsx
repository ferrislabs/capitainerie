import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { ClientDetailPage } from '#/domain/clients/feature/client-detail-page'
import { ClientsPage } from '#/domain/clients/feature/clients-page'
import { ProjectDetailPage } from '#/domain/clients/feature/project-detail-page'
import { ProjectsPage } from '#/domain/clients/feature/projects-page'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

vi.mock('@tanstack/react-router', () => ({
	Link: ({
		to,
		children,
		...rest
	}: {
		to: string
		children: ReactNode
		className?: string
	}) => (
		<a href={to} className={rest.className}>
			{children}
		</a>
	),
}))

const ACME: Schemas.ClientResponse = {
	id: 'cl1',
	name: 'Acme',
	aliases: ['ACME SA'],
	notes: 'Client historique',
	archived: false,
	created_by: 'ada',
	created_at: '2026-10-16T09:30:00Z',
	updated_at: '2026-10-16T09:30:00Z',
}

const SITE: Schemas.ProjectResponse = {
	id: 'p1',
	client: { id: 'cl1', name: 'Acme' },
	name: 'Site',
	aliases: [],
	repository: 'acme/web',
	description: 'Refonte',
	archived: false,
	created_by: 'ada',
	created_at: '2026-10-16T09:30:00Z',
	updated_at: '2026-10-16T09:30:00Z',
}

const COUNTS: Schemas.ColumnCountResponse[] = [
	{ column: 'proposed', cards: 1 },
	{ column: 'todo', cards: 2 },
	{ column: 'in_progress', cards: 0 },
	{ column: 'done', cards: 0 },
	{ column: 'rejected', cards: 0 },
]

const CARD = {
	id: 'c1',
	title: 'Relancer Acme',
	description: '',
	column: 'todo',
	provenance: null,
	created_by_run: null,
	created_by_subject: 'ada',
	version: 1,
	created_at: '2026-10-16T09:30:00Z',
	updated_at: '2026-10-16T09:30:00Z',
	client: { id: 'cl1', name: 'Acme' },
	project: { id: 'p1', name: 'Site' },
} as Schemas.CardResponse

const RUN = {
	id: 'r1',
	status: 'succeeded',
	transcript_id: 't1',
	transcript_title: 'Point Acme',
	title: 'Point Acme',
	card_id: null,
	repository: null,
	initiated_by: 'ada',
	pipeline: ['extract'],
	failure: null,
	created_at: '2026-10-16T09:00:00Z',
	started_at: null,
	finished_at: null,
	trace_id: null,
	client: { id: 'cl1', name: 'Acme' },
	project: null,
} as Schemas.RunResponse

type Route = (init: RequestInit, url: URL) => Response

describe('clients and projects pages', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function serve(table: Record<string, Route>) {
		fetchMock.mockImplementation((url: URL, init: RequestInit) => {
			const route = table[`${init.method} ${url.pathname}`]
			if (!route) throw new Error(`unexpected ${init.method} ${url.pathname}`)
			return Promise.resolve(route(init, url))
		})
	}

	function calls(method: string, path: string) {
		return fetchMock.mock.calls.filter(
			([url, init]) =>
				(url as URL).pathname === path &&
				(init as RequestInit).method === method,
		)
	}

	const base = {
		'GET /api/v1/clients': () =>
			json(200, {
				items: [{ client: ACME, project_count: 2, open_cards: 3 }],
			}),
		'GET /api/v1/projects': () =>
			json(200, { items: [{ project: SITE, open_cards: 1 }] }),
		'GET /api/v1/cards': () => json(200, { items: [CARD] }),
		'GET /api/v1/runs': () => json(200, { items: [RUN], total: 1 }),
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

	it('lists the clients as cards with their figures', async () => {
		serve(base)
		render(<ClientsPage />, { wrapper: harness.wrapper })

		const card = await screen.findByRole('link', { name: /Acme/ })

		expect(card.getAttribute('href')).toBe('/clients/$clientId')
		expect(card.textContent).toContain('2')
		expect(card.textContent).toContain('projets')
		expect(card.textContent).toContain('3')
		expect(card.textContent).toContain('cartes ouvertes')
		expect(card.textContent).toContain('ACME SA')
	})

	it('searches on the server', async () => {
		serve(base)
		render(<ClientsPage />, { wrapper: harness.wrapper })
		await screen.findByRole('link', { name: /Acme/ })

		await userEvent.type(
			screen.getByRole('searchbox', { name: 'Rechercher un client' }),
			'glo',
		)

		await waitFor(() =>
			expect(
				fetchMock.mock.calls.some(
					([url]) => (url as URL).searchParams.get('q') === 'glo',
				),
			).toBe(true),
		)
	})

	it('shows an empty state with its single action', async () => {
		serve({ ...base, 'GET /api/v1/clients': () => json(200, { items: [] }) })
		render(<ClientsPage />, { wrapper: harness.wrapper })

		expect(await screen.findByText('Aucun client')).toBeTruthy()
		expect(
			screen.getAllByRole('button', { name: 'Nouveau client' }),
		).toHaveLength(2)
	})

	it('creates a client from the dialog and refreshes the list', async () => {
		serve({
			...base,
			'POST /api/v1/clients': () => json(201, ACME),
		})
		render(<ClientsPage />, { wrapper: harness.wrapper })
		await screen.findByRole('link', { name: /Acme/ })
		const user = userEvent.setup()

		await user.click(
			screen.getAllByRole('button', { name: 'Nouveau client' })[0],
		)
		const dialog = await screen.findByRole('dialog')
		await user.type(within(dialog).getByLabelText('Nom'), ' Globex ')
		await user.type(within(dialog).getByLabelText('Alias'), 'GLX, Globex SA')
		await user.click(within(dialog).getByRole('button', { name: 'Créer' }))

		await waitFor(() =>
			expect(calls('POST', '/api/v1/clients')).toHaveLength(1),
		)
		const init = calls('POST', '/api/v1/clients')[0][1] as RequestInit
		expect(JSON.parse(String(init.body))).toEqual({
			name: 'Globex',
			aliases: ['GLX', 'Globex SA'],
			notes: '',
		})
		await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
		expect(calls('GET', '/api/v1/clients').length).toBeGreaterThan(1)
	})

	it('refuses an empty name and shows a duplicate as a conflict', async () => {
		serve({
			...base,
			'POST /api/v1/clients': () =>
				json(409, { code: 'conflict', status: 409, message: 'dup' }),
		})
		render(<ClientsPage />, { wrapper: harness.wrapper })
		await screen.findByRole('link', { name: /Acme/ })
		const user = userEvent.setup()
		await user.click(
			screen.getAllByRole('button', { name: 'Nouveau client' })[0],
		)
		const dialog = await screen.findByRole('dialog')

		await user.click(within(dialog).getByRole('button', { name: 'Créer' }))
		expect(
			await within(dialog).findByText('Le nom est obligatoire.'),
		).toBeTruthy()
		expect(calls('POST', '/api/v1/clients')).toHaveLength(0)

		await user.type(within(dialog).getByLabelText('Nom'), 'Acme')
		await user.click(within(dialog).getByRole('button', { name: 'Créer' }))
		expect(
			await within(dialog).findByText('Ce nom est déjà utilisé.'),
		).toBeTruthy()
	})

	it('lists the projects with their client', async () => {
		serve(base)
		render(<ProjectsPage />, { wrapper: harness.wrapper })

		const card = await screen.findByRole('link', { name: /Site/ })

		expect(card.getAttribute('href')).toBe('/projects/$projectId')
		expect(card.textContent).toContain('Acme')
	})

	it('shows a client with its projects, cards by column and runs', async () => {
		serve({
			...base,
			'GET /api/v1/clients/cl1': () =>
				json(200, { client: ACME, projects: [SITE], cards: COUNTS }),
		})
		render(<ClientDetailPage clientId="cl1" />, { wrapper: harness.wrapper })

		expect(await screen.findByRole('heading', { name: 'Acme' })).toBeTruthy()
		expect(screen.getByText('Aussi appelé : ACME SA')).toBeTruthy()
		expect(screen.getByRole('link', { name: /Site/ })).toBeTruthy()
		const todo = await screen.findByRole('region', { name: 'À faire' })
		expect(within(todo).getByText('2')).toBeTruthy()
		expect(
			within(todo).getByRole('link', { name: 'Relancer Acme' }),
		).toBeTruthy()
		expect(await screen.findByText('Point Acme')).toBeTruthy()
	})

	it('archives a client', async () => {
		serve({
			...base,
			'GET /api/v1/clients/cl1': () =>
				json(200, { client: ACME, projects: [], cards: COUNTS }),
			'PATCH /api/v1/clients/cl1': () => json(200, { ...ACME, archived: true }),
		})
		render(<ClientDetailPage clientId="cl1" />, { wrapper: harness.wrapper })

		await userEvent.click(
			await screen.findByRole('button', { name: 'Archiver' }),
		)

		await waitFor(() =>
			expect(calls('PATCH', '/api/v1/clients/cl1')).toHaveLength(1),
		)
		const init = calls('PATCH', '/api/v1/clients/cl1')[0][1] as RequestInit
		expect(JSON.parse(String(init.body))).toEqual({ archived: true })
	})

	it('says so when the client does not exist', async () => {
		serve({
			...base,
			'GET /api/v1/clients/nope': () =>
				json(404, { code: 'not_found', status: 404, message: 'not found' }),
		})
		render(<ClientDetailPage clientId="nope" />, { wrapper: harness.wrapper })

		expect(await screen.findByText('Client introuvable')).toBeTruthy()
	})

	it('edits a project and clears its repository with null', async () => {
		serve({
			...base,
			'GET /api/v1/projects/p1': () =>
				json(200, { project: SITE, cards: COUNTS }),
			'PATCH /api/v1/projects/p1': () => json(200, SITE),
		})
		render(<ProjectDetailPage projectId="p1" />, { wrapper: harness.wrapper })
		const user = userEvent.setup()

		await user.click(await screen.findByRole('button', { name: 'Modifier' }))
		const dialog = await screen.findByRole('dialog')
		await user.clear(within(dialog).getByLabelText('Dépôt par défaut'))
		await user.click(
			within(dialog).getByRole('button', { name: 'Enregistrer' }),
		)

		await waitFor(() =>
			expect(calls('PATCH', '/api/v1/projects/p1')).toHaveLength(1),
		)
		const init = calls('PATCH', '/api/v1/projects/p1')[0][1] as RequestInit
		expect(JSON.parse(String(init.body))).toMatchObject({
			name: 'Site',
			client_id: 'cl1',
			repository: null,
		})
	})
})
