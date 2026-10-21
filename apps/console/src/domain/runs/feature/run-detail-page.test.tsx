import { render, screen } from '@testing-library/react'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { RunDetailPage } from '#/domain/runs/feature/run-detail-page'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

vi.mock('@tanstack/react-router', () => ({
	Link: ({ to, children }: { to: string; children: ReactNode }) => (
		<a href={to}>{children}</a>
	),
}))

const RUN: Schemas.RunResponse = {
	id: 'r5',
	status: 'succeeded',
	card_id: 'c1',
	client: { id: 'cl1', name: 'Acme' },
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

function timeline(run: Schemas.RunResponse): Schemas.RunTimelineResponse {
	return {
		run,
		steps: [],
		decisions: [],
		totals: { input_tokens: 0, output_tokens: 0, cost_micros: 0 },
	}
}

const ACTION: Schemas.IntegrationActionResponse = {
	id: 'a1',
	run_id: 'r5',
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
	outcome: 'succeeded',
	revertible: false,
	created_at: '2026-10-16T12:00:30Z',
}

describe('RunDetailPage', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function serve(run: Schemas.RunResponse) {
		fetchMock.mockImplementation((url: URL) => {
			if (url.pathname === '/api/v1/runs/r5/timeline') {
				return Promise.resolve(json(200, timeline(run)))
			}
			if (url.pathname === '/api/v1/runs/r5/integration-actions') {
				return Promise.resolve(json(200, { items: [ACTION] }))
			}
			throw new Error(`unexpected ${url.pathname}`)
		})
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

	it('titles an intervention run and lists its journal', async () => {
		serve(RUN)
		render(<RunDetailPage runId="r5" />, { wrapper: harness.wrapper })

		expect(
			await screen.findByRole('heading', { name: 'Intervention pour Acme' }),
		).toBeTruthy()
		expect(screen.getByText('Intervention')).toBeTruthy()
		expect(
			await screen.findByRole('heading', { name: 'Journal des intégrations' }),
		).toBeTruthy()
		expect(screen.getByText('Odoo · compter')).toBeTruthy()
		expect(screen.getByText('res.partner')).toBeTruthy()
	})

	it('has no journal on an execution run', async () => {
		serve({ ...RUN, pipeline: ['execute'], repository: 'acme/web' })
		render(<RunDetailPage runId="r5" />, { wrapper: harness.wrapper })

		await screen.findByRole('heading', { name: 'Exécution sur acme/web' })

		expect(screen.getByText('Exécution')).toBeTruthy()
		expect(
			screen.queryByRole('heading', { name: 'Journal des intégrations' }),
		).toBeNull()
		expect(
			fetchMock.mock.calls.some(([url]) =>
				(url as URL).pathname.endsWith('/integration-actions'),
			),
		).toBe(false)
	})
})
