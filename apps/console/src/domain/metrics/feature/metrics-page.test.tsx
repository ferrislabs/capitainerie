import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { MetricsPage } from '#/domain/metrics/feature/metrics-page'
import { METRICS_TABS, type MetricsTab } from '#/domain/metrics/tabs'
import { clearAuth } from '#/store/auth.store'
import {
	type MockRoutes,
	mockApi,
	withQueryClient,
} from '#/test-utils/api-mock'

const AGENT_ID = 'a0000000-0000-4000-8000-000000000001'
const CARD_ID = 'c0000000-0000-4000-8000-000000000001'

const figures = {
	input_tokens: 12_000,
	output_tokens: 3_000,
	cost_micros: 1_500_000,
}
const total = figures

const BASE: MockRoutes = {
	'GET /api/v1/team/agents': {
		items: [{ id: AGENT_ID, name: 'Iris (Relais retours)', hue: 120 }],
	},
	'GET /api/v1/cards': {
		items: [{ id: CARD_ID, title: 'Relancer les factures' }],
	},
	'GET /api/v1/analytics/costs/projects': {
		from: '2026-10-19T00:00:00Z',
		to: '2026-11-18T00:00:00Z',
		total,
		rows: [
			{
				project_id: null,
				name: null,
				...figures,
				agents: [
					{
						agent_id: AGENT_ID,
						name: 'Iris (Relais retours)',
						runs: 2,
						...figures,
					},
				],
			},
		],
	},
	'GET /api/v1/analytics/costs/days': {
		from: '2026-10-19T00:00:00Z',
		to: '2026-11-18T00:00:00Z',
		total,
		rows: [{ day: '2026-11-10', ...figures }],
	},
	'GET /api/v1/analytics/costs/users': {
		from: '2026-10-19T00:00:00Z',
		to: '2026-11-18T00:00:00Z',
		total,
		rows: [],
	},
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

async function open(tab: MetricsTab, routes: MockRoutes = BASE, query = '') {
	const fetchMock = mockApi(routes)
	await renderAtPath(
		withQueryClient(<MetricsPage tab={tab} />),
		`/metrics/${tab}${query}`,
		['/metrics/{-$tab}'],
	)
	await screen.findByRole('heading', { name: 'Métriques' })
	return fetchMock
}

const urls = (fetchMock: ReturnType<typeof mockApi>, path: string) =>
	fetchMock.mock.calls
		.map(([input]) => new URL(String(input)))
		.filter((url) => url.pathname === path)

describe('MetricsPage', () => {
	it('renders one link per tab', async () => {
		await open('projets')
		const nav = screen.getByRole('navigation', {
			name: 'Sections des métriques',
		})
		expect(within(nav).getAllByRole('link')).toHaveLength(METRICS_TABS.length)
	})

	it('shows the API total and unfolds a row on its agents', async () => {
		await open('projets')
		expect((await screen.findAllByText('Sans projet')).length).toBeGreaterThan(
			0,
		)
		expect(screen.getAllByText(/1,50/).length).toBeGreaterThan(0)
		expect(
			screen.queryByRole('cell', { name: 'Iris (Relais retours)' }),
		).toBeNull()
		await userEvent.click(screen.getByRole('button', { name: /Sans projet/ }))
		expect(
			screen.getByRole('cell', { name: 'Iris (Relais retours)' }),
		).toBeTruthy()
	})

	it('reloads with agent_id when an agent is chosen', async () => {
		const fetchMock = await open('projets')
		await screen.findAllByText('Sans projet')
		await userEvent.selectOptions(
			screen.getByLabelText('Filtrer par agent'),
			AGENT_ID,
		)
		await waitFor(() => {
			const last = urls(fetchMock, '/api/v1/analytics/costs/projects').at(-1)
			expect(last?.searchParams.get('agent_id')).toBe(AGENT_ID)
		})
	})

	it('shows an empty state when nothing was spent', async () => {
		await open('utilisateurs')
		expect(
			await screen.findByText('Aucune dépense sur cette période'),
		).toBeTruthy()
	})

	it('explains a refused read', async () => {
		await open('projets', {
			...BASE,
			'GET /api/v1/analytics/costs/projects': {
				status: 403,
				body: { code: 'forbidden', message: 'no', status: 403 },
			},
		})
		expect(
			await screen.findByText(
				"Tu n'as pas accès aux métriques de l'organisation.",
			),
		).toBeTruthy()
	})

	it('loads the range picked on the calendar, the end day included', async () => {
		const fetchMock = await open('projets')
		await screen.findAllByText('Sans projet')
		await userEvent.click(screen.getByRole('button', { name: /^Période/ }))
		const start = screen.getAllByRole('button', {
			name: /^\S+ 1 \S+ \d{4}$/,
		})[0]
		const end = screen.getAllByRole('button', { name: /^\S+ 3 \S+ \d{4}$/ })[0]
		if (!start || !end) throw new Error('days not found')
		await userEvent.click(start)
		await userEvent.click(end)
		const previous = new Date()
		previous.setUTCDate(1)
		previous.setUTCMonth(previous.getUTCMonth() - 1)
		const month = previous.toISOString().slice(0, 7)
		await waitFor(() => {
			const last = urls(fetchMock, '/api/v1/analytics/costs/projects').at(-1)
			expect(last?.searchParams.get('from')).toBe(`${month}-01T00:00:00.000Z`)
			expect(last?.searchParams.get('to')).toBe(`${month}-04T00:00:00.000Z`)
		})
	})

	it('applies a preset from the picker', async () => {
		await open('projets')
		await screen.findAllByText('Sans projet')
		await userEvent.click(screen.getByRole('button', { name: /^Période/ }))
		await userEvent.click(
			screen.getByRole('button', { name: '7 derniers jours' }),
		)
		expect(
			screen.getByRole('button', { name: 'Période : 7 derniers jours' }),
		).toBeTruthy()
	})

	it('compares the cost with the previous period and offers a cumulative view', async () => {
		const fetchMock = await open('projets')
		await screen.findAllByText('Sans projet')
		expect(await screen.findByText(/vs période précédente/)).toBeTruthy()
		expect(urls(fetchMock, '/api/v1/analytics/costs/days')).toHaveLength(2)
		const cumulative = screen.getByRole('button', { name: 'Cumulé' })
		expect(cumulative.getAttribute('aria-pressed')).toBe('false')
		await userEvent.click(cumulative)
		expect(cumulative.getAttribute('aria-pressed')).toBe('true')
		await userEvent.click(
			screen.getByRole('button', { name: 'Voir le tableau' }),
		)
		expect(screen.getByRole('columnheader', { name: 'Jour' })).toBeTruthy()
	})

	it('reads the period and filters from the URL', async () => {
		const fetchMock = await open(
			'projets',
			BASE,
			`?periode=libre&du=2026-11-01&au=2026-11-03&agent=${AGENT_ID}&tache=${CARD_ID}`,
		)
		await screen.findAllByText('Sans projet')
		const last = urls(fetchMock, '/api/v1/analytics/costs/projects').at(-1)
		expect(last?.searchParams.get('from')).toBe('2026-11-01T00:00:00.000Z')
		expect(last?.searchParams.get('to')).toBe('2026-11-04T00:00:00.000Z')
		expect(last?.searchParams.get('agent_id')).toBe(AGENT_ID)
		expect(last?.searchParams.get('card_id')).toBe(CARD_ID)
		expect(
			(screen.getByLabelText('Filtrer par agent') as HTMLSelectElement).value,
		).toBe(AGENT_ID)
	})
})
