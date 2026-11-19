import { fireEvent, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { TasksPage } from '#/domain/tasks/feature/tasks-page'
import type { TasksSearch } from '#/domain/tasks/task-search'
import {
	AGENT_LEAD,
	AGENTS_ROUTE,
	agent,
	card,
	jsonBody,
} from '#/domain/tasks/test-fixtures'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'

const EXTRA = ['/tasks', '/tasks/$taskId', '/agents/$agentId/{-$tab}']

const ID_A = '11111111-aaaa-4000-8000-000000000001'
const ID_B = '22222222-bbbb-4000-8000-000000000002'
const ID_C = '33333333-cccc-4000-8000-000000000003'

const BASE = {
	...AGENTS_ROUTE,
	'GET /api/v1/projects': {
		items: [
			{
				project: {
					id: 'p1',
					name: 'Site vitrine',
					description: '',
					aliases: [],
					archived: false,
					created_at: '2026-11-01T00:00:00Z',
					updated_at: '2026-11-01T00:00:00Z',
					created_by: 'u',
				},
				open_cards: 1,
			},
		],
	},
	'GET /api/v1/cards': {
		items: [
			card(ID_A, {
				title: 'Alpha',
				column: 'in_progress',
				assignee_agent_id: AGENT_LEAD,
				project: { id: 'p1', name: 'Site vitrine' },
			}),
			card(ID_B, { title: 'Beta', column: 'todo' }),
			card(ID_C, { title: 'Gamma', column: 'done' }),
		],
	},
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

const render = (search: Partial<TasksSearch> = {}) =>
	renderAtPath(
		withQueryClient(<TasksPage search={{ vue: 'liste', q: '', ...search }} />),
		'/tasks',
		EXTRA,
	)

describe('TasksPage', () => {
	it('lists the cards grouped by column with short ids and agents', async () => {
		mockApi(BASE)
		await render()
		expect(await screen.findByText('Alpha')).toBeTruthy()
		expect(screen.getByRole('heading', { name: 'Tâches' })).toBeTruthy()
		expect(screen.getByText('2 ouvertes')).toBeTruthy()
		expect(screen.getByRole('region', { name: 'En cours' })).toBeTruthy()
		expect(screen.getByRole('region', { name: 'Terminé' })).toBeTruthy()
		expect(screen.queryByRole('region', { name: 'Bloqué' })).toBeNull()
		const link = screen
			.getAllByRole('link')
			.find((l) => l.textContent?.includes('11111111'))
		expect(link?.getAttribute('href')).toBe(`/tasks/${ID_A}`)
		await waitFor(() => expect(screen.getByText('Veille légale')).toBeTruthy())
		expect(
			screen.getByRole('link', { name: 'Liste' }).getAttribute('aria-current'),
		).toBe('page')
	})

	it('renders the board with the four real columns', async () => {
		mockApi(BASE)
		await render({ vue: 'tableau' })
		await screen.findByText('Alpha')
		expect(screen.getByRole('region', { name: 'Proposé' })).toBeTruthy()
		expect(screen.getByRole('region', { name: 'Terminé' })).toBeTruthy()
		expect(screen.queryByRole('region', { name: 'Rejeté' })).toBeNull()
		expect(
			screen
				.getByRole('link', { name: 'Tableau' })
				.getAttribute('aria-current'),
		).toBe('page')
	})

	it('filters with the search field', async () => {
		mockApi(BASE)
		await render()
		await screen.findByText('Alpha')
		fireEvent.change(
			screen.getByRole('textbox', { name: 'Rechercher des tâches' }),
			{ target: { value: 'beta' } },
		)
		expect(screen.queryByText('Alpha')).toBeNull()
		expect(screen.getByText('Beta')).toBeTruthy()
	})

	it('applies the status, agent and project filters from the URL', async () => {
		mockApi(BASE)
		await render({ statut: 'progress' })
		expect(await screen.findByText('Alpha')).toBeTruthy()
		expect(screen.queryByText('Beta')).toBeNull()
		expect(screen.queryByText('Gamma')).toBeNull()
	})

	it('keeps the unassigned and project filters', async () => {
		mockApi(BASE)
		const { unmount } = await render({ agent: 'aucun' })
		expect(await screen.findByText('Beta')).toBeTruthy()
		expect(screen.queryByText('Alpha')).toBeNull()
		unmount()
		await render({ projet: 'p1' })
		expect(await screen.findByText('Alpha')).toBeTruthy()
		expect(screen.queryByText('Beta')).toBeNull()
	})

	it('writes the chosen filter, sort and grouping to the URL', async () => {
		mockApi(BASE)
		const { router } = await render()
		await screen.findByText('Alpha')
		fireEvent.click(screen.getByRole('button', { name: 'Filtrer' }))
		fireEvent.change(screen.getByLabelText('Statut'), {
			target: { value: 'done' },
		})
		await waitFor(() =>
			expect(router.state.location.search).toMatchObject({ statut: 'done' }),
		)
		fireEvent.click(screen.getByRole('button', { name: 'Trier' }))
		fireEvent.change(screen.getByLabelText('Trier par'), {
			target: { value: 'titre' },
		})
		await waitFor(() =>
			expect(router.state.location.search).toMatchObject({ tri: 'titre' }),
		)
		fireEvent.click(screen.getByRole('button', { name: 'Grouper' }))
		fireEvent.change(screen.getByLabelText('Grouper par'), {
			target: { value: 'agent' },
		})
		await waitFor(() =>
			expect(router.state.location.search).toMatchObject({ groupe: 'agent' }),
		)
	})

	it('groups by agent and by project', async () => {
		mockApi(BASE)
		const { unmount } = await render({ groupe: 'agent' })
		expect(
			await screen.findByRole('region', { name: 'Veille légale' }),
		).toBeTruthy()
		expect(screen.getByRole('region', { name: 'Non assignée' })).toBeTruthy()
		unmount()
		await render({ groupe: 'projet' })
		expect(
			await screen.findByRole('region', { name: 'Site vitrine' }),
		).toBeTruthy()
		expect(screen.getByRole('region', { name: 'Sans projet' })).toBeTruthy()
	})

	it('sorts by title', async () => {
		mockApi(BASE)
		await render({ groupe: 'projet', tri: 'titre' })
		const region = await screen.findByRole('region', { name: 'Sans projet' })
		const text = region.textContent ?? ''
		expect(text.indexOf('Beta')).toBeLessThan(text.indexOf('Gamma'))
	})

	it('shows an empty state', async () => {
		mockApi({ ...BASE, 'GET /api/v1/cards': { items: [] } })
		await render()
		expect(await screen.findByText('Aucune tâche')).toBeTruthy()
	})

	it('shows an error when the list fails', async () => {
		mockApi({
			...BASE,
			'GET /api/v1/cards': {
				status: 500,
				body: { code: 'x', message: 'x', status: 500 },
			},
		})
		await render()
		expect(
			await screen.findByText('Impossible de charger les tâches.'),
		).toBeTruthy()
	})

	it('creates a card from the dialog and sends the chosen fields', async () => {
		const created = card('44444444-dddd-4000-8000-000000000004', {
			title: 'Ma tâche',
		})
		const fetchMock = mockApi({
			...BASE,
			'POST /api/v1/cards': { status: 201, body: created },
		})
		await render()
		fireEvent.click(
			await screen.findByRole('button', { name: 'Nouvelle tâche' }),
		)
		fireEvent.click(
			await screen.findByRole('button', { name: 'Créer la tâche' }),
		)
		expect(await screen.findByText('Écris le titre de la tâche')).toBeTruthy()
		fireEvent.change(screen.getByLabelText('Titre'), {
			target: { value: 'Ma tâche' },
		})
		await waitFor(() =>
			expect(screen.getByRole('option', { name: 'Site vitrine' })).toBeTruthy(),
		)
		fireEvent.change(screen.getByLabelText('Projet'), {
			target: { value: 'p1' },
		})
		fireEvent.change(screen.getByLabelText('Priorité'), {
			target: { value: 'high' },
		})
		fireEvent.change(screen.getByLabelText('Assigner à'), {
			target: { value: AGENT_LEAD },
		})
		fireEvent.change(screen.getByLabelText('Tâche parente'), {
			target: { value: ID_B },
		})
		expect(screen.getByText(/commence dès la création/)).toBeTruthy()
		fireEvent.click(screen.getByRole('button', { name: 'Créer et lancer' }))
		await waitFor(() =>
			expect(jsonBody(fetchMock, 'POST', '/api/v1/cards')).toEqual({
				title: 'Ma tâche',
				description: null,
				priority: 'high',
				assignee_agent_id: AGENT_LEAD,
				project_id: 'p1',
				parent_id: ID_B,
				repository: null,
			}),
		)
		await waitFor(() =>
			expect(
				screen.queryByRole('button', { name: 'Créer et lancer' }),
			).toBeNull(),
		)
	})

	it('asks for the repository when the agent has several and no connector', async () => {
		const fetchMock = mockApi({
			...BASE,
			'GET /api/v1/team/agents': {
				items: [
					agent(AGENT_LEAD, 'Auditeur', {
						repository_grants: ['acme/web', 'acme/api'],
					}),
				],
			},
			'POST /api/v1/cards': {
				status: 201,
				body: card('44444444-dddd-4000-8000-000000000004'),
			},
		})
		await render()
		fireEvent.click(
			await screen.findByRole('button', { name: 'Nouvelle tâche' }),
		)
		fireEvent.change(await screen.findByLabelText('Titre'), {
			target: { value: 'Auditer' },
		})
		await waitFor(() =>
			expect(screen.getByRole('option', { name: 'Auditeur' })).toBeTruthy(),
		)
		expect(screen.queryByLabelText('Dépôt')).toBeNull()
		fireEvent.change(screen.getByLabelText('Assigner à'), {
			target: { value: AGENT_LEAD },
		})
		fireEvent.change(screen.getByLabelText('Dépôt'), {
			target: { value: 'acme/api' },
		})
		fireEvent.click(screen.getByRole('button', { name: 'Créer et lancer' }))
		await waitFor(() =>
			expect(jsonBody(fetchMock, 'POST', '/api/v1/cards')).toMatchObject({
				assignee_agent_id: AGENT_LEAD,
				repository: 'acme/api',
			}),
		)
	})

	it('shows the API error when the creation fails', async () => {
		mockApi({
			...BASE,
			'POST /api/v1/cards': {
				status: 422,
				body: { code: 'invalid', message: 'x', status: 422 },
			},
		})
		await render()
		fireEvent.click(
			await screen.findByRole('button', { name: 'Nouvelle tâche' }),
		)
		fireEvent.change(await screen.findByLabelText('Titre'), {
			target: { value: 'X' },
		})
		fireEvent.click(screen.getByRole('button', { name: 'Créer la tâche' }))
		expect(
			await screen.findByText('Les informations de la tâche sont invalides.'),
		).toBeTruthy()
	})

	it('closes the dialog on cancel without creating', async () => {
		const fetchMock = mockApi(BASE)
		await render()
		fireEvent.click(
			await screen.findByRole('button', { name: 'Nouvelle tâche' }),
		)
		fireEvent.click(await screen.findByRole('button', { name: 'Annuler' }))
		expect(screen.queryByRole('button', { name: 'Créer la tâche' })).toBeNull()
		expect(jsonBody(fetchMock, 'POST', '/api/v1/cards')).toBeUndefined()
	})
})
