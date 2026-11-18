import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { ProjectsPage } from '#/domain/projects/feature/projects-page'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'

const project = (
	id: string,
	name: string,
	extra: Record<string, unknown> = {},
) => ({
	id,
	name,
	description: `Description ${name}`,
	aliases: [],
	archived: false,
	created_at: '2026-11-01T00:00:00Z',
	updated_at: '2026-11-01T00:00:00Z',
	created_by: 'u',
	...extra,
})

const card = (id: string, projectId: string, column: string) => ({
	id,
	column,
	title: id,
	description: '',
	labels: [],
	acceptance_criteria: [],
	priority: 'normal',
	created_at: '2026-11-01T00:00:00Z',
	updated_at: '2026-11-01T00:00:00Z',
	created_by_subject: 'u',
	version: 1,
	project: { id: projectId, name: projectId },
})

const BASE = {
	'GET /api/v1/projects': {
		items: [
			{
				project: project('p1', 'Site vitrine', {
					repository: 'acme/site',
					client: { id: 'cl1', name: 'Acme' },
				}),
				open_cards: 3,
			},
			{ project: project('p2', 'Appli mobile'), open_cards: 0 },
		],
	},
	'GET /api/v1/clients': {
		items: [
			{
				client: {
					id: 'cl1',
					name: 'Acme',
					aliases: [],
					archived: false,
					created_at: '2026-11-01T00:00:00Z',
					updated_at: '2026-11-01T00:00:00Z',
					created_by: 'u',
					notes: '',
				},
				open_cards: 0,
				project_count: 1,
			},
		],
	},
	'GET /api/v1/cards': {
		items: [
			card('a', 'p1', 'done'),
			card('b', 'p1', 'todo'),
			card('c', 'p1', 'todo'),
			card('d', 'p1', 'rejected'),
		],
	},
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

function renderPage() {
	return render(withQueryClient(<ProjectsPage />))
}

describe('ProjectsPage', () => {
	it('lists the projects of the API with progress from the cards', async () => {
		mockApi(BASE)
		renderPage()
		expect(await screen.findByText('Site vitrine')).toBeTruthy()
		expect(screen.getByRole('heading', { name: 'Projets' })).toBeTruthy()
		expect(screen.getByText('2 projets actifs')).toBeTruthy()
		expect(screen.getAllByRole('article')).toHaveLength(2)
		expect(screen.getByText('acme/site')).toBeTruthy()
		expect(screen.getByText('Acme')).toBeTruthy()
		await waitFor(() =>
			expect(
				screen
					.getByRole('progressbar', { name: 'Avancement de Site vitrine' })
					.getAttribute('aria-valuenow'),
			).toBe('33'),
		)
		expect(screen.getByText('33 % · 3 ouvertes')).toBeTruthy()
		expect(
			screen.queryByRole('progressbar', { name: 'Avancement de Appli mobile' }),
		).toBeNull()
		expect(screen.getByText('0 ouverte')).toBeTruthy()
	})

	it('shows an empty state', async () => {
		mockApi({ ...BASE, 'GET /api/v1/projects': { items: [] } })
		renderPage()
		expect(await screen.findByText('Aucun projet')).toBeTruthy()
	})

	it('shows an error when the list fails', async () => {
		mockApi({
			...BASE,
			'GET /api/v1/projects': {
				status: 500,
				body: { code: 'x', message: 'x', status: 500 },
			},
		})
		renderPage()
		expect(
			await screen.findByText('Impossible de charger les projets.'),
		).toBeTruthy()
	})

	it('creates a project and refreshes the list', async () => {
		const fetchMock = mockApi({
			...BASE,
			'POST /api/v1/projects': {
				status: 201,
				body: project('p3', 'Nouveau'),
			},
		})
		renderPage()
		await screen.findByText('Site vitrine')
		await userEvent.click(
			screen.getByRole('button', { name: 'Nouveau projet' }),
		)
		const dialog = await screen.findByRole('dialog')
		await userEvent.click(
			within(dialog).getByRole('button', { name: 'Créer le projet' }),
		)
		expect(within(dialog).getByText('Donne un nom au projet')).toBeTruthy()
		await userEvent.type(within(dialog).getByLabelText('Nom'), ' Nouveau ')
		await userEvent.type(
			within(dialog).getByLabelText('Propriétaire du dépôt'),
			'acme',
		)
		await userEvent.type(within(dialog).getByLabelText('Nom du dépôt'), 'neuf')
		await userEvent.selectOptions(
			within(dialog).getByLabelText('Client (facultatif)'),
			'cl1',
		)
		await userEvent.click(
			within(dialog).getByRole('button', { name: 'Créer le projet' }),
		)
		await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
		const post = fetchMock.mock.calls.find(
			([, init]) => init?.method === 'POST',
		)
		expect(JSON.parse(String(post?.[1]?.body))).toEqual({
			name: 'Nouveau',
			description: '',
			repository: 'acme/neuf',
			client_id: 'cl1',
		})
		const lists = fetchMock.mock.calls.filter(
			([url, init]) =>
				new URL(String(url)).pathname === '/api/v1/projects' &&
				(init?.method ?? 'GET') === 'GET',
		)
		expect(lists.length).toBeGreaterThan(1)
	})

	it('reports a duplicate name returned by the API', async () => {
		mockApi({
			...BASE,
			'POST /api/v1/projects': {
				status: 409,
				body: { code: 'conflict', message: 'dup', status: 409 },
			},
		})
		renderPage()
		await screen.findByText('Site vitrine')
		await userEvent.click(
			screen.getByRole('button', { name: 'Nouveau projet' }),
		)
		const dialog = await screen.findByRole('dialog')
		await userEvent.type(within(dialog).getByLabelText('Nom'), 'Site vitrine')
		await userEvent.click(
			within(dialog).getByRole('button', { name: 'Créer le projet' }),
		)
		expect(
			await within(dialog).findByText('Un projet porte déjà ce nom.'),
		).toBeTruthy()
	})

	it('patches only the edited fields', async () => {
		const fetchMock = mockApi({
			...BASE,
			'PATCH /api/v1/projects/p1': project('p1', 'Site refait'),
		})
		renderPage()
		await screen.findByText('Site vitrine')
		await userEvent.click(
			screen.getAllByRole('button', { name: 'Modifier' })[0] as HTMLElement,
		)
		const dialog = await screen.findByRole('dialog')
		const name = within(dialog).getByLabelText('Nom')
		expect((name as HTMLInputElement).value).toBe('Site vitrine')
		expect(
			(within(dialog).getByLabelText('Nom du dépôt') as HTMLInputElement).value,
		).toBe('site')
		await userEvent.clear(name)
		await userEvent.type(name, 'Site refait')
		await userEvent.click(
			within(dialog).getByRole('button', { name: 'Enregistrer' }),
		)
		await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
		const patch = fetchMock.mock.calls.find(
			([, init]) => init?.method === 'PATCH',
		)
		expect(JSON.parse(String(patch?.[1]?.body))).toEqual({
			name: 'Site refait',
		})
	})

	it('archives a project with PATCH archived', async () => {
		const fetchMock = mockApi({
			...BASE,
			'PATCH /api/v1/projects/p2': project('p2', 'Appli mobile', {
				archived: true,
			}),
		})
		renderPage()
		await screen.findByText('Appli mobile')
		const article = screen
			.getByText('Appli mobile')
			.closest('article') as HTMLElement
		await userEvent.click(
			within(article).getByRole('button', { name: 'Archiver' }),
		)
		await waitFor(() => {
			const patch = fetchMock.mock.calls.find(
				([, init]) => init?.method === 'PATCH',
			)
			expect(JSON.parse(String(patch?.[1]?.body))).toEqual({ archived: true })
		})
	})

	it('asks the API for archived projects', async () => {
		const fetchMock = mockApi(BASE)
		renderPage()
		await screen.findByText('Site vitrine')
		await userEvent.click(
			screen.getByRole('button', { name: 'Voir les archivés' }),
		)
		await waitFor(() =>
			expect(
				fetchMock.mock.calls.some(([url]) =>
					String(url).includes('/api/v1/projects?archived=true'),
				),
			).toBe(true),
		)
	})
})
