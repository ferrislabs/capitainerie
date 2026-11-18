import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { DashboardPage } from '#/domain/dashboard/feature/dashboard-page'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { CHIEF, LEGAL } from '#/test-utils/team-agents'

const EXTRA = [
	'/agents/$agentId/{-$tab}',
	'/audit/{-$tab}',
	'/tasks',
	'/tasks/$taskId',
]

const NOW = Date.now()
const ago = (minutes: number) => new Date(NOW - minutes * 60_000).toISOString()
const today = new Date().toISOString().slice(0, 10)

const run = (id: string, extra: Record<string, unknown> = {}) => ({
	id,
	status: 'succeeded',
	initiated_by: 'user-1',
	pipeline: [],
	created_at: ago(30),
	title: `Run ${id}`,
	...extra,
})

const card = (id: string, column: string, title: string, minutes: number) => ({
	id,
	column,
	title,
	description: '',
	labels: [],
	acceptance_criteria: [],
	priority: 'normal',
	created_at: ago(minutes),
	updated_at: ago(minutes),
	created_by_subject: 'u',
	version: 1,
	project: { id: 'p1', name: 'Site vitrine' },
})

const FULL = {
	'GET /api/v1/runs/active': {
		items: [
			{
				run: run('r1', {
					status: 'running',
					card_id: 'c-aaa',
					title: 'Revue de code',
					started_at: ago(5),
				}),
				cost_micros: 0,
				input_tokens: 0,
				output_tokens: 0,
				steps_done: 1,
				steps_total: 3,
				current_step: { agent: 'Iris', kind: 'agent', started_at: ago(1) },
			},
			{
				run: run('r2', { status: 'queued', created_at: ago(120), title: null }),
				cost_micros: 0,
				input_tokens: 0,
				output_tokens: 0,
				steps_done: 0,
				steps_total: 2,
			},
		],
	},
	'GET /api/v1/runs': {
		items: [
			run('r3', { status: 'failed', failure: 'timeout du build' }),
			run('r4'),
		],
		total: 42,
	},
	'GET /api/v1/analytics/costs': {
		rows: [
			{
				key: today,
				runs: 6,
				input_tokens: 1500,
				output_tokens: 500,
				cost_micros: 2_500_000,
			},
		],
		total: { input_tokens: 1500, output_tokens: 500, cost_micros: 2_500_000 },
	},
	'GET /api/v1/cards': {
		items: [
			card('c1', 'in_progress', 'Corriger le SEO', 10),
			card('c2', 'todo', 'Rédiger la newsletter', 20),
			card('c3', 'proposed', 'Idée de refonte', 30),
			card('c4', 'done', 'Livrer le site', 40),
		],
	},
	'GET /api/v1/approvals': { items: [], pending: 3 },
	'GET /api/v1/inbox': { items: [], total: 12, unread: 4 },
	'GET /api/v1/team/agents': { items: [CHIEF, LEGAL] },
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

async function renderDashboard(routes: Parameters<typeof mockApi>[0]) {
	const fetchMock = mockApi(routes)
	await renderAtPath(withQueryClient(<DashboardPage />), '/', EXTRA)
	await screen.findByRole('heading', { name: 'Tableau de bord' })
	return fetchMock
}

describe('DashboardPage', () => {
	it('computes the header and the tiles from the API', async () => {
		await renderDashboard(FULL)
		expect(
			await screen.findByText('2 agents · 2 exécutions actives'),
		).toBeTruthy()
		const tile = (label: string) =>
			screen.getByText(label).closest('a') as HTMLElement
		await waitFor(() => {
			expect(within(tile('Agents')).getByText('2')).toBeTruthy()
		})
		expect(within(tile('Agents')).getByText('1 en cours')).toBeTruthy()
		expect(within(tile('Tâches en cours')).getByText('1')).toBeTruthy()
		expect(within(tile('Tâches en cours')).getByText('2 ouvertes')).toBeTruthy()
		expect(within(tile('À valider')).getByText('1')).toBeTruthy()
		expect(within(tile('Dépense sur 14 jours')).getByText(/2,50/)).toBeTruthy()
		expect(
			within(tile('Dépense sur 14 jours')).getByText(/2 k jetons/),
		).toBeTruthy()
		expect(
			within(tile('Exécutions actives')).getByText('1 en file'),
		).toBeTruthy()
		expect(within(tile('Approbations')).getByText('3')).toBeTruthy()
		expect(within(tile('Non lus')).getByText('4')).toBeTruthy()
		expect(tile('Approbations').getAttribute('href')).toBe('/approvals')
		expect(tile('Non lus').getAttribute('href')).toBe('/inbox/non-lus')
	})

	it('renders the live runs and links the card of a run', async () => {
		await renderDashboard(FULL)
		expect(await screen.findByText('Revue de code')).toBeTruthy()
		expect(screen.getByText('Étape 1/3 · il y a 5 min')).toBeTruthy()
		expect(screen.getByText('Exécution en cours')).toBeTruthy()
		const link = screen.getByText('Revue de code').closest('a')
		expect(link?.getAttribute('href')).toBe('/tasks/c-aaa')
		expect(
			screen.getByRole('link', { name: '42 exécutions au total' }),
		).toBeTruthy()
	})

	it('draws the charts from the daily costs', async () => {
		await renderDashboard(FULL)
		for (const title of [
			'Activité des exécutions',
			'Jetons par jour',
			'Coût par jour',
		]) {
			expect(
				await screen.findByRole('img', { name: `${title}, 14 derniers jours` }),
			).toBeTruthy()
		}
	})

	it('lists what needs attention, recent activity and recent tasks', async () => {
		await renderDashboard(FULL)
		expect(await screen.findByText('timeout du build')).toBeTruthy()
		expect(screen.getByText('1 exécution récente en échec')).toBeTruthy()
		expect(screen.getByText('1 tâche proposée à valider')).toBeTruthy()
		expect(screen.getByText('Run r3 · en échec')).toBeTruthy()
		const link = await screen.findByRole('link', { name: /Corriger le SEO/ })
		expect(link.getAttribute('href')).toBe('/tasks/c1')
		expect(within(link).getByText('il y a 10 min')).toBeTruthy()
		expect(screen.getByText('Livrer le site')).toBeTruthy()
	})

	it('shows empty states instead of numbers without data', async () => {
		await renderDashboard({
			'GET /api/v1/runs/active': { items: [] },
			'GET /api/v1/runs': { items: [], total: 0 },
			'GET /api/v1/analytics/costs': {
				rows: [],
				total: { input_tokens: 0, output_tokens: 0, cost_micros: 0 },
			},
			'GET /api/v1/cards': { items: [] },
			'GET /api/v1/approvals': { items: [], pending: 0 },
			'GET /api/v1/inbox': { items: [], total: 0, unread: 0 },
			'GET /api/v1/team/agents': { items: [] },
		})
		expect(
			await screen.findByText("Aucun agent en cours d'exécution."),
		).toBeTruthy()
		expect(screen.getAllByText('Aucune activité sur la période')).toHaveLength(
			3,
		)
		expect(screen.getByText('Rien à traiter pour le moment.')).toBeTruthy()
		expect(screen.getByText('Aucune exécution pour le moment.')).toBeTruthy()
		expect(screen.getByText('Aucune tâche pour le moment.')).toBeTruthy()
		expect(screen.queryByRole('img')).toBeNull()
	})

	it('shows errors without inventing figures', async () => {
		const failure = {
			status: 500,
			body: { code: 'x', message: 'x', status: 500 },
		}
		await renderDashboard({
			'GET /api/v1/runs/active': failure,
			'GET /api/v1/runs': failure,
			'GET /api/v1/analytics/costs': failure,
			'GET /api/v1/cards': failure,
			'GET /api/v1/approvals': failure,
			'GET /api/v1/inbox': failure,
			'GET /api/v1/team/agents': failure,
		})
		expect(
			await screen.findByText('Impossible de charger les exécutions en cours.'),
		).toBeTruthy()
		expect(screen.getAllByText('Données indisponibles')).toHaveLength(3)
		for (const label of ['Agents', 'Approbations', 'Non lus']) {
			const tile = screen.getByText(label, { selector: 'span' }).closest('a')
			expect(within(tile as HTMLElement).getByText('–')).toBeTruthy()
		}
	})

	it('shows a loading state before the responses', async () => {
		mockApi(FULL)
		vi.stubGlobal('fetch', () => new Promise(() => {}))
		await renderAtPath(withQueryClient(<DashboardPage />), '/', EXTRA)
		await screen.findByRole('heading', { name: 'Tableau de bord' })
		expect(screen.getAllByText('Chargement…').length).toBeGreaterThan(0)
		expect(screen.queryByText('0')).toBeNull()
	})

	it('links to the pages and to the costs tab', async () => {
		await renderDashboard(FULL)
		await screen.findByText('Revue de code')
		const hrefs = screen.getAllByRole('link').map((l) => l.getAttribute('href'))
		expect(hrefs).toContain('/agents')
		expect(hrefs).toContain('/audit/couts')
		expect(hrefs).toContain('/audit/runs')
		expect(
			screen
				.getByRole('link', { name: "Journal d'audit" })
				.getAttribute('href'),
		).toBe('/audit')
	})

	it('links the task tiles to the task list', async () => {
		await renderDashboard(FULL)
		for (const label of ['Tâches en cours', 'À valider']) {
			const a = screen.getByText(label).closest('a')
			expect(a?.getAttribute('href')?.startsWith('/tasks')).toBe(true)
		}
	})

	it('opens the new task dialog', async () => {
		await renderDashboard(FULL)
		await userEvent.click(
			screen.getByRole('button', { name: 'Nouvelle tâche' }),
		)
		expect(await screen.findByRole('dialog')).toBeTruthy()
	})
})
