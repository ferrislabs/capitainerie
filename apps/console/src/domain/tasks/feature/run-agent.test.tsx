import { fireEvent, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { TaskPage } from '#/domain/tasks/feature/task-page'
import {
	AGENT_LEAD,
	AGENTS_ROUTE,
	card,
	jsonBody,
} from '#/domain/tasks/test-fixtures'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'

const EXTRA = [
	'/tasks',
	'/tasks/$taskId',
	'/agents/$agentId/{-$tab}',
	'/connectors/{-$tab}',
	'/providers',
]
const ID = '11111111-aaaa-4000-8000-000000000001'
const RUN = '99999999-aaaa-4000-8000-000000000009'
const POST = `POST /api/v1/cards/${ID}/agent-runs`

const detail = (extra = {}, comments: unknown[] = []) => ({
	card: card(ID, { title: 'Tâche', assignee_agent_id: AGENT_LEAD, ...extra }),
	comments,
	runs: [],
})

const run = (status: string, failure?: string) => ({
	id: RUN,
	status,
	failure,
	created_at: new Date().toISOString(),
	initiated_by: 'u',
	pipeline: [],
})

const base = (card = detail()) => ({
	...AGENTS_ROUTE,
	[`GET /api/v1/cards/${ID}`]: card,
	'GET /api/v1/cards': { items: [] },
})

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
	document.getElementById('topbar-actions')?.remove()
})

const open = () =>
	renderAtPath(withQueryClient(<TaskPage pathId={ID} />), `/tasks/${ID}`, EXTRA)

const launch = async () => {
	fireEvent.click(await screen.findByRole('button', { name: /Exécuter avec/ }))
	fireEvent.click(await screen.findByRole('button', { name: 'Confirmer' }))
}

describe('run agent', () => {
	it('hides the action without assignee or when done', async () => {
		mockApi(base(detail({ assignee_agent_id: undefined })))
		await open()
		await screen.findByRole('heading', { level: 1 })
		expect(screen.queryByRole('button', { name: /Exécuter avec/ })).toBeNull()
	})

	it('hides the action for a done card', async () => {
		mockApi(base(detail({ column: 'done' })))
		await open()
		await screen.findByRole('heading', { level: 1 })
		expect(screen.queryByRole('button', { name: /Exécuter avec/ })).toBeNull()
	})

	it('opens the dialog with writes off and posts the body', async () => {
		const fetchMock = mockApi({
			...base(),
			[POST]: { status: 202, body: { run_id: RUN } },
			[`GET /api/v1/runs/${RUN}`]: run('running'),
		})
		await open()
		fireEvent.click(
			await screen.findByRole('button', { name: /Exécuter avec/ }),
		)
		const box = await screen.findByRole('checkbox', {
			name: /Autoriser les écritures/,
		})
		expect(box.getAttribute('aria-checked')).toBe('false')
		fireEvent.click(screen.getByRole('button', { name: 'Confirmer' }))
		expect(await screen.findByText('En cours')).toBeTruthy()
		expect(
			jsonBody(fetchMock, 'POST', `/api/v1/cards/${ID}/agent-runs`),
		).toEqual({ expected_version: 3, allow_writes: false })
		await waitFor(() =>
			expect(
				(
					screen.getByRole('button', {
						name: /Exécuter avec/,
					}) as HTMLButtonElement
				).disabled,
			).toBe(true),
		)
	})

	it('shows the not_connected message with a connectors link', async () => {
		mockApi({
			...base(),
			[POST]: {
				status: 409,
				body: {
					code: 'not_connected',
					message: "le connecteur mestier n'est pas connecté",
					status: 409,
				},
			},
		})
		await open()
		await launch()
		expect(
			await screen.findByText(/le connecteur mestier n'est pas connecté/),
		).toBeTruthy()
		const link = screen.getByRole('link', { name: 'Ouvrir les connecteurs' })
		expect(link.getAttribute('href')).toBe('/connectors/installes')
	})

	it('shows the no_provider message with a providers link', async () => {
		mockApi({
			...base(),
			[POST]: {
				status: 409,
				body: {
					code: 'no_provider',
					message: 'Aucun fournisseur LLM connecté pour votre compte',
					status: 409,
				},
			},
		})
		await open()
		await launch()
		expect(
			await screen.findByText(/Aucun fournisseur LLM connecté/),
		).toBeTruthy()
		const link = screen.getByRole('link', { name: 'Connecter un fournisseur' })
		expect(link.getAttribute('href')).toBe('/providers')
	})

	it('shows the agent_paused refusal', async () => {
		mockApi({
			...base(),
			[POST]: {
				status: 422,
				body: {
					code: 'agent_paused',
					message: "l'agent assigné est en pause",
					status: 422,
				},
			},
		})
		await open()
		await launch()
		expect(
			await screen.findByText(/L'agent assigné est en pause : reprends-le/),
		).toBeTruthy()
	})

	it('polls to success and refetches the card', async () => {
		let polls = 0
		const fetchMock = mockApi({
			...base(),
			[POST]: { status: 202, body: { run_id: RUN } },
			[`GET /api/v1/runs/${RUN}/timeline`]: {
				run: run('succeeded'),
				decisions: [],
				totals: { cost_micros: 0, input_tokens: 0, output_tokens: 0 },
				steps: [
					{
						seq: 1,
						tool_calls: [
							{ tool: 'lire', summary: 'ok', succeeded: true, latency_ms: 1 },
						],
					},
				],
			},
		})
		const inner = fetchMock.getMockImplementation()
		fetchMock.mockImplementation(async (input, init) => {
			const url = new URL(String(input))
			if (
				url.pathname === `/api/v1/runs/${RUN}` &&
				(init?.method ?? 'GET') === 'GET'
			) {
				polls += 1
				return new Response(
					JSON.stringify(run(polls < 2 ? 'running' : 'succeeded')),
					{ status: 200, headers: { 'Content-Type': 'application/json' } },
				)
			}
			return (inner as NonNullable<typeof inner>)(input, init)
		})
		await open()
		await launch()
		expect(
			await screen.findByText('Terminée', {}, { timeout: 6000 }),
		).toBeTruthy()
		expect(await screen.findByText(/lire : ok/)).toBeTruthy()
		await waitFor(() => {
			const cardCalls = fetchMock.mock.calls.filter(
				([u, i]) =>
					new URL(String(u)).pathname === `/api/v1/cards/${ID}` &&
					((i as RequestInit | undefined)?.method ?? 'GET') === 'GET',
			)
			expect(cardCalls.length).toBeGreaterThan(1)
		})
	}, 10000)

	it('shows the failure message', async () => {
		mockApi({
			...base(),
			[POST]: { status: 202, body: { run_id: RUN } },
			[`GET /api/v1/runs/${RUN}`]: run('failed', 'quota dépassé'),
		})
		await open()
		await launch()
		expect(await screen.findByText('Échec')).toBeTruthy()
		expect(screen.getByText('quota dépassé')).toBeTruthy()
	})
})
