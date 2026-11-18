import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { ApprovalsPage } from '#/domain/approvals/feature/approvals-page'
import { approval } from '#/domain/approvals/test-fixtures'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { DEV, TEAM } from '#/test-utils/team-agents'

const EXTRA = ['/tasks', '/tasks/$taskId']
const CARD = 'C0FFEE00-0000-4000-8000-000000000001'

const SEND = approval('ap-1', {
	title: 'Envoyer le devis',
	requested_by_agent_id: DEV.id,
	card_id: CARD,
	detail: {
		tool: 'gmail.send',
		arguments: { to: 'client@example.com', subject: 'Devis' },
	},
})

type Routes = Parameters<typeof mockApi>[0]

async function open(routes: Routes = {}) {
	const fetchMock = mockApi({
		'GET /api/v1/approvals': { items: [SEND], pending: 1 },
		'GET /api/v1/team/agents': { items: TEAM },
		...routes,
	})
	await renderAtPath(withQueryClient(<ApprovalsPage />), '/approvals', EXTRA)
	return fetchMock
}

function calls(fetchMock: ReturnType<typeof mockApi>, method: string) {
	return fetchMock.mock.calls
		.filter(([, init]) => init?.method === method)
		.map(([url, init]) => ({
			url: new URL(String(url)),
			body: init?.body ? JSON.parse(String(init.body)) : undefined,
		}))
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

describe('ApprovalsPage', () => {
	it('renders a tool call with its tool, arguments, agent and task', async () => {
		await open()
		expect(
			screen.getByRole('heading', { name: 'Approbations', level: 1 }),
		).toBeTruthy()
		const card = await screen.findByRole('article', {
			name: 'Envoyer le devis',
		})
		expect(within(card).getByText('gmail.send')).toBeTruthy()
		expect(within(card).getByLabelText('Arguments').textContent).toContain(
			'"subject": "Devis"',
		)
		expect(within(card).getByText(/demandée par Dev/)).toBeTruthy()
		const link = within(card).getByRole('link', { name: /Tâche c0ffee00/i })
		expect(link.getAttribute('href')).toBe(`/tasks/${CARD.toLowerCase()}`)
		expect(screen.queryByText(/démonstration/i)).toBeNull()
	})

	it('shows the empty state', async () => {
		await open({ 'GET /api/v1/approvals': { items: [], pending: 0 } })
		expect(
			await screen.findByText('Aucune approbation en attente'),
		).toBeTruthy()
	})

	it('switches status tab and queries the API with it', async () => {
		const user = userEvent.setup()
		const fetchMock = await open()
		await screen.findByRole('article')
		await user.click(screen.getByRole('button', { name: 'Refusées' }))
		await waitFor(() =>
			expect(
				calls(fetchMock, 'GET').some(
					(c) =>
						c.url.pathname === '/api/v1/approvals' &&
						c.url.searchParams.get('status') === 'rejected',
				),
			).toBe(true),
		)
		expect(
			screen
				.getByRole('button', { name: 'Refusées' })
				.getAttribute('aria-pressed'),
		).toBe('true')
	})

	it('approves with a note and links the resumed run to its task', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			'POST /api/v1/approvals/ap-1/decide': {
				...SEND,
				status: 'approved',
				resumed_run_id: 'run-2',
				resume_error: null,
			},
		})
		await screen.findByRole('article')
		await user.type(screen.getByLabelText('Note (optionnelle)'), 'OK pour moi')
		await user.click(
			screen.getByRole('button', { name: 'Approuver : Envoyer le devis' }),
		)
		expect(await screen.findByText(/Exécution relancée/)).toBeTruthy()
		const link = screen.getByRole('link', { name: 'Voir la tâche' })
		expect(link.getAttribute('href')).toBe(`/tasks/${CARD.toLowerCase()}`)
		expect(calls(fetchMock, 'POST')[0]).toMatchObject({
			body: { decision: 'approve', note: 'OK pour moi' },
		})
	})

	it('says why an approved call did not start a new run', async () => {
		const user = userEvent.setup()
		await open({
			'POST /api/v1/approvals/ap-1/decide': {
				...SEND,
				status: 'approved',
				resumed_run_id: null,
				resume_error:
					"l'agent assigné est en pause : reprenez-le avant de le lancer",
			},
		})
		await user.click(
			await screen.findByRole('button', {
				name: 'Approuver : Envoyer le devis',
			}),
		)
		expect(
			await screen.findByText(
				/n'a pas été relancée : l'agent assigné est en pause/,
			),
		).toBeTruthy()
		expect(screen.getByRole('link', { name: 'Voir la tâche' })).toBeTruthy()
	})

	it('rejects without a note', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			'POST /api/v1/approvals/ap-1/decide': {
				...SEND,
				status: 'rejected',
				resume_error: null,
			},
		})
		await user.click(
			await screen.findByRole('button', {
				name: 'Refuser : Envoyer le devis',
			}),
		)
		expect(
			await screen.findByText(
				/Demande refusée. Le refus est noté sur la tâche./,
			),
		).toBeTruthy()
		expect(calls(fetchMock, 'POST')[0]?.body).toEqual({ decision: 'reject' })
	})

	it('explains a 409 when the approval is no longer pending', async () => {
		const user = userEvent.setup()
		await open({
			'POST /api/v1/approvals/ap-1/decide': {
				status: 409,
				body: { code: 'conflict', message: 'not pending', status: 409 },
			},
		})
		await user.click(
			await screen.findByRole('button', {
				name: 'Approuver : Envoyer le devis',
			}),
		)
		expect((await screen.findByRole('alert')).textContent).toMatch(
			/n'est plus en attente/,
		)
	})

	it('shows the decision of a decided approval', async () => {
		await open({
			'GET /api/v1/approvals': {
				items: [
					approval('ap-2', {
						title: 'Écrire dans le Drive',
						kind: 'agent_action',
						detail: { path: 'compta/' },
						status: 'rejected',
						decided_by: 'Baptiste',
						decided_at: '2026-11-18T10:00:00Z',
						decision_note: 'Pas maintenant',
					}),
				],
				pending: 0,
			},
		})
		const card = await screen.findByRole('article')
		expect(within(card).getByText(/Refusée par Baptiste/)).toBeTruthy()
		expect(within(card).getByText('« Pas maintenant »')).toBeTruthy()
		expect(within(card).getByLabelText('Détail').textContent).toContain(
			'compta/',
		)
		expect(within(card).queryByRole('button')).toBeNull()
	})
})
