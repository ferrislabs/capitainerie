import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { AuditPage } from '#/domain/audit/feature/audit-page'
import { AUDIT_TABS, type AuditTab } from '#/domain/audit/tabs'
import { clearAuth } from '#/store/auth.store'
import {
	type MockRoutes,
	mockApi,
	withQueryClient,
} from '#/test-utils/api-mock'

const EXTRA = ['/audit/{-$tab}', '/tasks/$taskId']
const NOW = Date.now()
const ago = (minutes: number) => new Date(NOW - minutes * 60_000).toISOString()

const AGENT_ID = 'a0000000-0000-4000-8000-000000000001'

const event = (id: string, extra: Record<string, unknown> = {}) => ({
	id,
	topic: 'card',
	type: 'card.moved',
	occurred_at: ago(5),
	actor: { kind: 'agent', id: AGENT_ID, name: 'Iris (Relais retours)' },
	summary: 'a déplacé la tâche',
	severity: 'info',
	payload: {},
	...extra,
})

const budget = (extra: Record<string, unknown> = {}) => ({
	scope: 'organization',
	monthly_limit_cents: 40_000,
	alert_threshold_percent: 80,
	spent_micros: 100_000_000,
	percent: 25,
	alert: false,
	exceeded: false,
	updated_by: 'admin',
	updated_at: ago(60),
	...extra,
})

const BUDGETS = {
	month_start: '2026-11-01',
	month_end: '2026-12-01',
	spent_micros: 100_000_000,
	items: [
		budget(),
		budget({
			scope: 'agent',
			agent_id: AGENT_ID,
			agent_name: 'Iris (Relais retours)',
			monthly_limit_cents: 10_000,
			percent: 100,
			alert: true,
			exceeded: true,
		}),
	],
}

const BASE: MockRoutes = {
	'GET /api/v1/team/agents': {
		items: [{ id: AGENT_ID, name: 'Iris (Relais retours)', hue: 120 }],
	},
	'GET /api/v1/audit/events': {
		items: [
			event('e1'),
			event('e2', {
				actor: { kind: 'member', id: 'u1', name: 'Baptiste Parmantier' },
				summary: 'a lu la tâche NAT-679',
				severity: 'critical',
			}),
		],
		next_cursor: 'cursor-2',
	},
	'GET /api/v1/runs': {
		items: [
			{
				id: 'r1',
				status: 'failed',
				initiated_by: 'u1',
				pipeline: [],
				created_at: ago(30),
				started_at: ago(30),
				finished_at: ago(27),
				title: 'Sonde de mise en page',
				failure: 'timeout du build',
				card_id: 'c1',
				agent: { id: AGENT_ID, name: 'Iris (Relais retours)' },
			},
		],
		total: 1,
	},
	'GET /api/v1/analytics/costs': {
		rows: [
			{
				key: AGENT_ID,
				runs: 4,
				input_tokens: 12_000,
				output_tokens: 3_000,
				cost_micros: 1_500_000,
			},
		],
		total: {
			input_tokens: 12_000,
			output_tokens: 3_000,
			cost_micros: 1_500_000,
		},
	},
	'GET /api/v1/budgets': BUDGETS,
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

async function open(tab: AuditTab, routes: MockRoutes = BASE) {
	const fetchMock = mockApi(routes)
	await renderAtPath(
		withQueryClient(<AuditPage tab={tab} />),
		`/audit/${tab}`,
		EXTRA,
	)
	await screen.findByRole('heading', { name: 'Audit' })
	return fetchMock
}

const calls = (fetchMock: ReturnType<typeof mockApi>, path: string) =>
	fetchMock.mock.calls
		.map(([input, init]) => ({ url: new URL(String(input)), init }))
		.filter(({ url }) => url.pathname === path)

describe('AuditPage', () => {
	it('renders the tabs without any demo notice', async () => {
		await open('activite')
		const nav = screen.getByRole('navigation', { name: "Sections de l'audit" })
		expect(within(nav).getAllByRole('link')).toHaveLength(AUDIT_TABS.length)
		expect(screen.queryByText(/démonstration/i)).toBeNull()
	})

	it('lists the audit events and loads the next page with the cursor', async () => {
		const fetchMock = await open('activite')
		expect(await screen.findByText('a lu la tâche NAT-679')).toBeTruthy()
		expect(screen.getByText('Baptiste Parmantier')).toBeTruthy()
		await userEvent.click(screen.getByRole('button', { name: 'Charger plus' }))
		await waitFor(() =>
			expect(calls(fetchMock, '/api/v1/audit/events')).toHaveLength(2),
		)
		const last = calls(fetchMock, '/api/v1/audit/events')[1]
		expect(last?.url.searchParams.get('cursor')).toBe('cursor-2')
	})

	it('shows an empty journal', async () => {
		await open('activite', {
			...BASE,
			'GET /api/v1/audit/events': { items: [], next_cursor: null },
		})
		expect(await screen.findByText('Aucun événement')).toBeTruthy()
		expect(screen.queryByRole('button', { name: 'Charger plus' })).toBeNull()
	})

	it('renders the timeline with severity markers', async () => {
		await open('chronologie')
		expect(await screen.findByText('a lu la tâche NAT-679')).toBeTruthy()
		expect(screen.getByRole('img', { name: 'Critique' })).toBeTruthy()
		expect(screen.getByRole('img', { name: 'Information' })).toBeTruthy()
	})

	it('renders the runs with their agent and filters them', async () => {
		const fetchMock = await open('runs')
		expect(await screen.findByText('Sonde de mise en page')).toBeTruthy()
		expect(screen.getByText('Iris')).toBeTruthy()
		expect(screen.getByText('échec', { selector: 'span' })).toBeTruthy()
		expect(screen.getByText('3 min 00')).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'Sonde de mise en page' })
				.getAttribute('href'),
		).toBe('/tasks/c1')
		await userEvent.selectOptions(
			screen.getByRole('combobox', { name: 'Filtrer par statut' }),
			'failed',
		)
		await userEvent.selectOptions(
			screen.getByRole('combobox', { name: 'Filtrer par agent' }),
			AGENT_ID,
		)
		await waitFor(() => {
			const last = calls(fetchMock, '/api/v1/runs').at(-1)
			expect(last?.url.searchParams.get('status')).toBe('failed')
			expect(last?.url.searchParams.get('agent_id')).toBe(AGENT_ID)
		})
	})

	it('renders the cost tiles and the by-agent table without cache column', async () => {
		const fetchMock = await open('couts')
		expect(await screen.findByText('Dépense du mois')).toBeTruthy()
		expect(screen.getByText('25 %')).toBeTruthy()
		expect(await screen.findByText('Iris')).toBeTruthy()
		expect(
			screen.queryByRole('columnheader', { name: 'Dont cache' }),
		).toBeNull()
		const costs = calls(fetchMock, '/api/v1/analytics/costs')[0]
		expect(costs?.url.searchParams.get('by')).toBe('agent')
	})

	it('renders one budget row per scope', async () => {
		await open('budgets')
		expect(await screen.findByText('Organisation')).toBeTruthy()
		expect(screen.getAllByRole('progressbar')).toHaveLength(2)
		expect(screen.getByText('Dépassé')).toBeTruthy()
	})

	it('saves the whole budget set', async () => {
		const fetchMock = await open('budgets', {
			...BASE,
			'PUT /api/v1/budgets': BUDGETS,
		})
		await userEvent.click(
			await screen.findByRole('button', { name: 'Modifier les budgets' }),
		)
		const limit = screen.getByRole('textbox', {
			name: 'Limite mensuelle en dollars : Organisation',
		})
		await userEvent.clear(limit)
		await userEvent.type(limit, '500')
		await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))
		await waitFor(() =>
			expect(
				calls(fetchMock, '/api/v1/budgets').some(
					({ init }) => init?.method === 'PUT',
				),
			).toBe(true),
		)
		const put = calls(fetchMock, '/api/v1/budgets').find(
			({ init }) => init?.method === 'PUT',
		)
		expect(JSON.parse(String(put?.init?.body))).toEqual({
			organization: {
				monthly_limit_cents: 50_000,
				alert_threshold_percent: 80,
			},
			agents: [
				{
					agent_id: AGENT_ID,
					monthly_limit_cents: 10_000,
					alert_threshold_percent: 80,
				},
			],
		})
		expect(
			await screen.findByRole('button', { name: 'Modifier les budgets' }),
		).toBeTruthy()
	})

	it('explains that only admins can change budgets', async () => {
		await open('budgets', {
			...BASE,
			'PUT /api/v1/budgets': {
				status: 403,
				body: { code: 'forbidden', message: 'forbidden', status: 403 },
			},
		})
		await userEvent.click(
			await screen.findByRole('button', { name: 'Modifier les budgets' }),
		)
		await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(
			await screen.findByText(
				'Seuls les administrateurs de la Capitainerie peuvent modifier les budgets.',
			),
		).toBeTruthy()
	})

	it('shows an error when the journal cannot load', async () => {
		await open('activite', {
			...BASE,
			'GET /api/v1/audit/events': {
				status: 500,
				body: { code: 'x', message: 'x', status: 500 },
			},
		})
		expect(
			await screen.findByText('Impossible de charger le journal.'),
		).toBeTruthy()
	})
})
