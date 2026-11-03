import { render, screen, waitFor, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { CardJournal } from '#/domain/integrations/journal/card-journal'
import { RunJournal } from '#/domain/integrations/journal/run-journal'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

function run(
	overrides: Partial<Schemas.RunResponse> = {},
): Schemas.RunResponse {
	return {
		id: 'r1',
		status: 'succeeded',
		initiated_by: 'ada',
		pipeline: ['intervene'],
		card_id: 'c1',
		created_at: '2026-10-16T09:30:00Z',
		...overrides,
	}
}

const ACTIONS: Array<Schemas.IntegrationActionResponse> = [
	{
		id: 'a1',
		run_id: 'r1',
		card_id: 'c1',
		instance_id: 'i1',
		instance_name: 'Odoo prod',
		instance_environment: 'production',
		instance_kind: 'odoo_json2',
		actor: 'capitainerie-agent-integrator',
		tool: 'odoo_count',
		access: 'read',
		target: { model: 'res.partner' },
		arguments: { model: 'res.partner' },
		outcome: 'denied',
		reason_code: 'instance_not_granted',
		revertible: false,
		created_at: '2026-10-16T09:31:00Z',
	},
]

describe('journal sections', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function serve(table: Record<string, () => Response>) {
		fetchMock.mockImplementation((url: URL, init: RequestInit) => {
			const route = table[`${init.method} ${url.pathname}`]
			if (!route) throw new Error(`unexpected ${init.method} ${url.pathname}`)
			return Promise.resolve(route())
		})
	}

	function refetchInterval(path: string) {
		const query = harness.client
			.getQueryCache()
			.getAll()
			.find((candidate) => {
				const head = candidate.queryKey[0] as { _id?: string }
				return head._id === path
			})
		return query?.observers[0]?.options.refetchInterval
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

	it('shows the journal of the card under its title', async () => {
		serve({
			'GET /api/v1/cards/c1/integration-actions': () =>
				json(200, { items: ACTIONS }),
		})
		render(<CardJournal cardId="c1" runs={[run()]} />, {
			wrapper: harness.wrapper,
		})

		const section = (
			await screen.findByRole('heading', { name: 'Journal des intégrations' })
		).closest('section') as HTMLElement
		expect(within(section).getByText('Odoo prod')).toBeTruthy()
		expect(
			within(section).getByText(
				'instance non autorisée pour cette intervention',
			),
		).toBeTruthy()
		expect(refetchInterval('/api/v1/cards/{card_id}/integration-actions')).toBe(
			false,
		)
	})

	it('polls the card journal while an intervention is queued or running', async () => {
		serve({
			'GET /api/v1/cards/c1/integration-actions': () =>
				json(200, { items: ACTIONS }),
		})
		render(
			<CardJournal
				cardId="c1"
				runs={[
					run({ id: 'r0', pipeline: ['execute'] }),
					run({ status: 'running' }),
				]}
			/>,
			{ wrapper: harness.wrapper },
		)

		await screen.findByText('Odoo prod')
		expect(refetchInterval('/api/v1/cards/{card_id}/integration-actions')).toBe(
			5000,
		)
	})

	it('stays out of the way without an intervention run', () => {
		const { container } = render(
			<CardJournal
				cardId="c1"
				runs={[run({ pipeline: ['execute'] }), run({ pipeline: ['extract'] })]}
			/>,
			{ wrapper: harness.wrapper },
		)

		expect(container.textContent).toBe('')
		expect(fetchMock).not.toHaveBeenCalled()
	})

	it('shows nothing while the run has no action yet', async () => {
		serve({
			'GET /api/v1/cards/c1/integration-actions': () =>
				json(200, { items: [] }),
		})
		const { container } = render(
			<CardJournal cardId="c1" runs={[run({ status: 'queued' })]} />,
			{ wrapper: harness.wrapper },
		)

		await waitFor(() => expect(fetchMock).toHaveBeenCalled())
		await waitFor(() =>
			expect(harness.client.getQueryCache().getAll()[0]?.state.status).toBe(
				'success',
			),
		)
		expect(container.textContent).toBe('')
	})

	it('says when the journal cannot be read', async () => {
		serve({
			'GET /api/v1/cards/c1/integration-actions': () =>
				json(403, { code: 'forbidden', status: 403, message: 'no' }),
		})
		render(<CardJournal cardId="c1" runs={[run()]} />, {
			wrapper: harness.wrapper,
		})

		expect(
			await screen.findByText(
				'Impossible de charger le journal des intégrations.',
			),
		).toBeTruthy()
	})

	it('shows the journal of an intervention run', async () => {
		serve({
			'GET /api/v1/runs/r1/integration-actions': () =>
				json(200, { items: ACTIONS }),
		})
		render(<RunJournal run={run()} />, { wrapper: harness.wrapper })

		expect(
			await screen.findByRole('heading', { name: 'Journal des intégrations' }),
		).toBeTruthy()
		expect(screen.getByText('Refusée')).toBeTruthy()
		expect(refetchInterval('/api/v1/runs/{run_id}/integration-actions')).toBe(
			false,
		)
	})

	it('polls the run journal while the run is active', async () => {
		serve({
			'GET /api/v1/runs/r1/integration-actions': () =>
				json(200, { items: ACTIONS }),
		})
		render(<RunJournal run={run({ status: 'queued' })} />, {
			wrapper: harness.wrapper,
		})

		await screen.findByText('Refusée')
		expect(refetchInterval('/api/v1/runs/{run_id}/integration-actions')).toBe(
			5000,
		)
	})

	it('ignores a run that is not an intervention', () => {
		const { container } = render(
			<RunJournal run={run({ pipeline: ['execute'] })} />,
			{ wrapper: harness.wrapper },
		)

		expect(container.textContent).toBe('')
		expect(fetchMock).not.toHaveBeenCalled()
	})
})
