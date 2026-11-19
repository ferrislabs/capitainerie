import { fireEvent, screen, waitFor, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { TaskPage } from '#/domain/tasks/feature/task-page'
import {
	AGENT_LEAD,
	AGENT_SUB,
	AGENTS_ROUTE,
	card,
	jsonBody,
} from '#/domain/tasks/test-fixtures'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'

const EXTRA = ['/tasks', '/tasks/$taskId', '/agents/$agentId/{-$tab}']

const ROOT = '11111111-aaaa-4000-8000-000000000001'
const KID_DONE = '22222222-bbbb-4000-8000-000000000002'
const KID_OPEN = '33333333-cccc-4000-8000-000000000003'
const PARENT = '44444444-dddd-4000-8000-000000000004'

const root = card(ROOT, {
	title: 'Tâche racine',
	description: 'Contexte détaillé',
	column: 'todo',
	assignee_agent_id: AGENT_LEAD,
	project: { id: 'p1', name: 'Site vitrine' },
	client: { id: 'c1', name: 'Acme' },
	labels: ['routine'],
	priority: 'high',
	version: 7,
})

const list = (items: ReturnType<typeof card>[]) => ({ items })

const BASE = {
	...AGENTS_ROUTE,
	[`GET /api/v1/cards/${ROOT}`]: {
		card: root,
		comments: [
			{
				id: 'cm1',
				author_subject: 'user-1',
				author_actor: 'Nathaël',
				body: 'Premier commentaire',
				created_at: new Date().toISOString(),
			},
		],
		runs: [],
	},
	'GET /api/v1/cards': list([
		root,
		card(KID_DONE, { title: 'Fille faite', column: 'done', parent_id: ROOT }),
		card(KID_OPEN, { title: 'Fille ouverte', parent_id: ROOT }),
	]),
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
	document.getElementById('topbar-actions')?.remove()
})

const open = (pathId: string) =>
	renderAtPath(
		withQueryClient(<TaskPage pathId={pathId} />),
		`/tasks/${pathId}`,
		EXTRA,
	)

describe('TaskPage', () => {
	it('renders the card, its comments, properties and sub-tasks', async () => {
		mockApi(BASE)
		await open(ROOT)
		expect(
			(await screen.findByRole('heading', { level: 1 })).textContent,
		).toBe('Tâche racine')
		expect(screen.getAllByText('11111111').length).toBeGreaterThan(0)
		expect(screen.getByText('Contexte détaillé')).toBeTruthy()
		expect(screen.getByText('Premier commentaire')).toBeTruthy()
		expect(screen.getByText('Nathaël')).toBeTruthy()
		expect(await screen.findByText('1 / 2 terminées')).toBeTruthy()
		const props = screen.getByRole('complementary', { name: 'Propriétés' })
		expect(within(props).getByText('Haute')).toBeTruthy()
		expect(within(props).getByText('Site vitrine')).toBeTruthy()
		expect(within(props).getByText('Acme')).toBeTruthy()
		expect(within(props).getByText('routine')).toBeTruthy()
		await waitFor(() =>
			expect(within(props).getByText('Veille légale')).toBeTruthy(),
		)
		expect(
			(screen.getByRole('combobox', { name: 'Statut' }) as HTMLSelectElement)
				.value,
		).toBe('todo')
	})

	it('shows the parent banner for a sub-task', async () => {
		mockApi({
			...BASE,
			[`GET /api/v1/cards/${KID_OPEN}`]: {
				card: card(KID_OPEN, { parent_id: ROOT, title: 'Fille ouverte' }),
				comments: [],
				runs: [],
			},
		})
		await open(KID_OPEN)
		expect(await screen.findByText(/Déléguée par/)).toBeTruthy()
		expect(
			screen
				.getAllByRole('link', { name: '11111111' })[0]
				?.getAttribute('href'),
		).toBe(`/tasks/${ROOT}`)
	})

	it('shows not found for a missing card', async () => {
		mockApi({ ...AGENTS_ROUTE, 'GET /api/v1/cards': list([]) })
		await open('99999999-0000-4000-8000-000000000009')
		expect(await screen.findByText('Tâche introuvable')).toBeTruthy()
		expect(screen.getByRole('link', { name: 'Retour aux tâches' })).toBeTruthy()
	})

	it('shows an error on a server failure', async () => {
		mockApi({
			...BASE,
			[`GET /api/v1/cards/${ROOT}`]: {
				status: 500,
				body: { code: 'x', message: 'x', status: 500 },
			},
		})
		await open(ROOT)
		expect(
			await screen.findByText('Impossible de charger la tâche.'),
		).toBeTruthy()
	})

	it('moves the card with the expected version and toggles the properties', async () => {
		const slot = document.createElement('div')
		slot.id = 'topbar-actions'
		document.body.append(slot)
		const fetchMock = mockApi({
			...BASE,
			[`POST /api/v1/cards/${ROOT}/move`]: {
				...root,
				column: 'in_progress',
				version: 8,
			},
		})
		await open(ROOT)
		fireEvent.change(await screen.findByRole('combobox', { name: 'Statut' }), {
			target: { value: 'progress' },
		})
		await waitFor(() =>
			expect(jsonBody(fetchMock, 'POST', `/api/v1/cards/${ROOT}/move`)).toEqual(
				{
					to: 'in_progress',
					expected_version: 7,
				},
			),
		)
		expect(await screen.findByText('Statut : En cours')).toBeTruthy()
		fireEvent.click(screen.getByRole('button', { name: 'Fermer' }))
		expect(screen.queryByRole('combobox', { name: 'Statut' })).toBeNull()
		fireEvent.click(
			screen.getByRole('button', { name: 'Afficher les propriétés' }),
		)
		expect(screen.getByRole('combobox', { name: 'Statut' })).toBeTruthy()
	})

	it('changes the assignee through the card update', async () => {
		const fetchMock = mockApi({
			...BASE,
			[`PATCH /api/v1/cards/${ROOT}`]: {
				...root,
				assignee_agent_id: AGENT_SUB,
				version: 8,
			},
		})
		await open(ROOT)
		const select = await screen.findByRole('combobox', { name: 'Assigné à' })
		await waitFor(() =>
			expect(
				within(select)
					.getAllByRole('option')
					.map((o) => o.textContent),
			).toEqual(['Personne', 'Veille légale', 'Registre']),
		)
		expect((select as HTMLSelectElement).value).toBe(AGENT_LEAD)
		fireEvent.change(select, { target: { value: AGENT_SUB } })
		await waitFor(() =>
			expect(jsonBody(fetchMock, 'PATCH', `/api/v1/cards/${ROOT}`)).toEqual({
				assignee_agent_id: AGENT_SUB,
				expected_version: 7,
			}),
		)
		expect(await screen.findByText('Assignée à Registre')).toBeTruthy()
		await waitFor(() =>
			expect(
				fetchMock.mock.calls.filter(
					([input]) =>
						new URL(String(input)).pathname === `/api/v1/cards/${ROOT}`,
				).length,
			).toBeGreaterThan(2),
		)
	})

	it('unassigns and reports a conflict', async () => {
		const fetchMock = mockApi({
			...BASE,
			[`PATCH /api/v1/cards/${ROOT}`]: {
				status: 409,
				body: { code: 'conflict', message: 'x', status: 409 },
			},
		})
		await open(ROOT)
		fireEvent.change(
			await screen.findByRole('combobox', { name: 'Assigné à' }),
			{ target: { value: '' } },
		)
		await waitFor(() =>
			expect(jsonBody(fetchMock, 'PATCH', `/api/v1/cards/${ROOT}`)).toEqual({
				assignee_agent_id: null,
				expected_version: 7,
			}),
		)
		expect(
			await screen.findByText('Cette tâche a changé entre-temps, réessaie'),
		).toBeTruthy()
	})

	it('reports a version conflict', async () => {
		mockApi({
			...BASE,
			[`POST /api/v1/cards/${ROOT}/move`]: {
				status: 409,
				body: { code: 'conflict', message: 'x', status: 409 },
			},
		})
		await open(ROOT)
		fireEvent.change(await screen.findByRole('combobox', { name: 'Statut' }), {
			target: { value: 'done' },
		})
		expect(
			await screen.findByText('Cette tâche a changé entre-temps, réessaie'),
		).toBeTruthy()
	})

	it('only offers the statuses backed by a column', async () => {
		mockApi(BASE)
		await open(ROOT)
		const select = await screen.findByRole('combobox', { name: 'Statut' })
		expect(
			within(select)
				.getAllByRole('option')
				.map((o) => o.textContent),
		).toEqual(['En cours', 'À faire', 'Proposé', 'Terminé', 'Rejeté'])
	})

	it('posts a comment through the API', async () => {
		const fetchMock = mockApi({
			...BASE,
			[`POST /api/v1/cards/${ROOT}/comments`]: {
				status: 201,
				body: {
					id: 'cm2',
					author_subject: 'user-1',
					body: 'Bonjour',
					created_at: new Date().toISOString(),
				},
			},
		})
		await open(ROOT)
		const field = await screen.findByRole('textbox', { name: 'Message' })
		fireEvent.change(field, { target: { value: 'Bonjour' } })
		fireEvent.click(screen.getByRole('button', { name: 'Envoyer' }))
		await waitFor(() =>
			expect(
				jsonBody(fetchMock, 'POST', `/api/v1/cards/${ROOT}/comments`),
			).toEqual({ body: 'Bonjour' }),
		)
		await waitFor(() => expect((field as HTMLTextAreaElement).value).toBe(''))
	})

	it('delegates a sub-task to a sub-agent as a child card', async () => {
		const fetchMock = mockApi({
			...BASE,
			'GET /api/v1/cards': list([root]),
			'POST /api/v1/cards': {
				status: 201,
				body: card(PARENT, { parent_id: ROOT }),
			},
		})
		await open(ROOT)
		fireEvent.change(
			await screen.findByRole('textbox', { name: 'Titre de la sous-tâche' }),
			{ target: { value: 'Vérifier le SIREN' } },
		)
		await waitFor(() =>
			expect(
				within(screen.getByRole('combobox', { name: 'Sous-agent' })).getByRole(
					'option',
					{ name: 'Registre' },
				),
			).toBeTruthy(),
		)
		fireEvent.click(screen.getByRole('button', { name: 'Déléguer' }))
		await waitFor(() =>
			expect(jsonBody(fetchMock, 'POST', '/api/v1/cards')).toEqual({
				title: 'Vérifier le SIREN',
				assignee_agent_id: AGENT_SUB,
				project_id: 'p1',
				parent_id: ROOT,
			}),
		)
		expect(await screen.findByText(/déléguée à Registre/)).toBeTruthy()
	})

	it('refuses past the parallel limit and an empty title', async () => {
		const fetchMock = mockApi({
			...BASE,
			'GET /api/v1/cards': list([
				root,
				card(KID_DONE, { parent_id: ROOT }),
				card(KID_OPEN, { parent_id: ROOT }),
				card(PARENT, { parent_id: ROOT }),
				card('55555555-eeee-4000-8000-000000000005', { parent_id: ROOT }),
			]),
		})
		await open(ROOT)
		const title = await screen.findByRole('textbox', {
			name: 'Titre de la sous-tâche',
		})
		fireEvent.click(await screen.findByRole('button', { name: 'Déléguer' }))
		expect(
			await screen.findByText('Écris le titre de la sous-tâche'),
		).toBeTruthy()
		fireEvent.change(title, { target: { value: 'Encore' } })
		fireEvent.click(screen.getByRole('button', { name: 'Déléguer' }))
		expect(
			await screen.findByText('Limite atteinte : 3 sous-tâches en parallèle'),
		).toBeTruthy()
		expect(jsonBody(fetchMock, 'POST', '/api/v1/cards')).toBeUndefined()
	})

	it('refuses to delegate past the depth limit', async () => {
		mockApi({
			...BASE,
			[`GET /api/v1/cards/${KID_OPEN}`]: {
				card: card(KID_OPEN, {
					parent_id: PARENT,
					assignee_agent_id: AGENT_LEAD,
				}),
				comments: [],
				runs: [],
			},
			'GET /api/v1/cards': list([
				card('66666666-ffff-4000-8000-000000000006', {}),
				card(PARENT, { parent_id: '66666666-ffff-4000-8000-000000000006' }),
				card(KID_OPEN, { parent_id: PARENT, assignee_agent_id: AGENT_LEAD }),
			]),
		})
		await open(KID_OPEN)
		fireEvent.change(
			await screen.findByRole('textbox', { name: 'Titre de la sous-tâche' }),
			{ target: { value: 'Trop profond' } },
		)
		fireEvent.click(screen.getByRole('button', { name: 'Déléguer' }))
		expect(
			await screen.findByText('Profondeur maximale atteinte (2)'),
		).toBeTruthy()
	})

	it('lists the artifacts of the task and opens one', async () => {
		const artifact = {
			id: 'art1',
			kind: 'markdown',
			title: 'Compte rendu',
			card_id: ROOT,
			created_by: 'agent',
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString(),
		}
		const fetchMock = mockApi({
			...BASE,
			'GET /api/v1/artifacts': { items: [artifact], next_cursor: null },
			'GET /api/v1/artifacts/art1': { ...artifact, content: '## Synthèse' },
		})
		await open(ROOT)
		fireEvent.click(await screen.findByRole('button', { name: /Compte rendu/ }))
		const dialog = await screen.findByRole('dialog')
		expect(
			await within(dialog).findByRole('heading', { name: 'Synthèse' }),
		).toBeTruthy()
		const request = fetchMock.mock.calls
			.map(([input]) => new URL(String(input)))
			.find((url) => url.pathname === '/api/v1/artifacts')
		expect(request?.searchParams.get('card_id')).toBe(ROOT)
	})
})
