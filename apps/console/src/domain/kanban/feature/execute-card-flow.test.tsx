import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { ExecuteCardFlow } from '#/domain/kanban/feature/execute-card-flow'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

vi.mock('@tanstack/react-router', () => ({
	Link: ({ to, children }: { to: string; children: ReactNode }) => (
		<a href={to}>{children}</a>
	),
}))

const CARD: Schemas.CardResponse = {
	id: 'c1',
	title: 'Corriger le bug',
	description: '',
	column: 'todo',
	provenance: null,
	created_by_run: null,
	created_by_subject: 'ada',
	created_by_agent: null,
	version: 5,
	created_at: '2026-10-16T09:30:00Z',
	updated_at: '2026-10-16T09:30:00Z',
}

const CONNECTED = {
	app: { slug: 'capi' },
	installation: { account_login: 'acme' },
}
const REPOSITORIES = {
	repositories: [{ full_name: 'acme/web', name: 'web', owner: 'acme' }],
}

type Route = (init: RequestInit) => Response

describe('ExecuteCardFlow', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function serve(table: Record<string, Route>) {
		fetchMock.mockImplementation((url: URL, init: RequestInit) => {
			const route = table[`${init.method} ${url.pathname}`]
			if (!route) throw new Error(`unexpected ${init.method} ${url.pathname}`)
			return Promise.resolve(route(init))
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

	it('sends the card version and the repository, then reports the run', async () => {
		const bodies: Array<unknown> = []
		serve({
			'GET /api/v1/integrations/github': () => json(200, CONNECTED),
			'GET /api/v1/integrations/github/repositories': () =>
				json(200, REPOSITORIES),
			'POST /api/v1/cards/c1/executions': (init) => {
				bodies.push(JSON.parse(String(init.body)))
				return json(202, { card: CARD, run_id: 'run-9' })
			},
		})
		const onStarted = vi.fn()
		const user = userEvent.setup()
		render(
			<ExecuteCardFlow card={CARD} onClose={vi.fn()} onStarted={onStarted} />,
			{
				wrapper: harness.wrapper,
			},
		)

		await user.selectOptions(await screen.findByLabelText('Dépôt'), 'acme/web')
		await user.click(screen.getByRole('button', { name: 'Confier' }))

		await waitFor(() => expect(onStarted).toHaveBeenCalledWith('run-9'))
		expect(bodies).toEqual([{ repository: 'acme/web', expected_version: 5 }])
	})

	it('shows the mapped message when the card already runs', async () => {
		serve({
			'GET /api/v1/integrations/github': () => json(200, CONNECTED),
			'GET /api/v1/integrations/github/repositories': () =>
				json(200, REPOSITORIES),
			'POST /api/v1/cards/c1/executions': () =>
				json(409, {
					code: 'conflict',
					status: 409,
					message: 'execution_already_active',
				}),
		})
		const onStarted = vi.fn()
		const user = userEvent.setup()
		render(
			<ExecuteCardFlow card={CARD} onClose={vi.fn()} onStarted={onStarted} />,
			{
				wrapper: harness.wrapper,
			},
		)

		await user.selectOptions(await screen.findByLabelText('Dépôt'), 'acme/web')
		await user.click(screen.getByRole('button', { name: 'Confier' }))

		expect(
			await screen.findByText(
				'Une exécution est déjà en cours pour cette carte.',
			),
		).toBeTruthy()
		expect(onStarted).not.toHaveBeenCalled()
	})

	it('asks to connect GitHub when the organisation has no installation', async () => {
		serve({
			'GET /api/v1/integrations/github': () =>
				json(200, { app: null, installation: null }),
		})
		render(
			<ExecuteCardFlow card={CARD} onClose={vi.fn()} onStarted={vi.fn()} />,
			{
				wrapper: harness.wrapper,
			},
		)

		expect(
			await screen.findByRole('link', { name: 'Paramètres › Intégrations' }),
		).toBeTruthy()
	})

	it('falls back to the 409 of the repositories endpoint', async () => {
		serve({
			'GET /api/v1/integrations/github': () => json(200, CONNECTED),
			'GET /api/v1/integrations/github/repositories': () =>
				json(409, {
					code: 'conflict',
					status: 409,
					message: 'github_not_connected',
				}),
		})
		render(
			<ExecuteCardFlow card={CARD} onClose={vi.fn()} onStarted={vi.fn()} />,
			{
				wrapper: harness.wrapper,
			},
		)

		expect(
			await screen.findByRole('link', { name: 'Paramètres › Intégrations' }),
		).toBeTruthy()
	})

	it('does not fetch while closed', () => {
		render(
			<ExecuteCardFlow
				card={undefined}
				onClose={vi.fn()}
				onStarted={vi.fn()}
			/>,
			{ wrapper: harness.wrapper },
		)

		expect(fetchMock).not.toHaveBeenCalled()
	})
})
