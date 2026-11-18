import { screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import {
	getAgentConfig,
	resetAgentConfigs,
} from '#/domain/agent-team/agent-config-store'
import { AgentWizardPage } from '#/domain/agent-team/feature/agent-wizard-page'
import { demoStore, resetDemoState } from '#/domain/demo'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { TEAM, teamAgent } from '#/test-utils/team-agents'

const EXTRA = ['/agents', '/agents/nouveau', '/agents/$agentId/{-$tab}']

const next = () => screen.getByRole('button', { name: /→$/ })

async function toLastStep(
	user: ReturnType<typeof userEvent.setup>,
	name: string,
) {
	await user.type(await screen.findByLabelText("Nom de l'agent"), name)
	await user.click(next())
	await user.click(screen.getByRole('button', { name: 'Codex' }))
	await user.click(next())
	await user.click(next())
	await user.click(next())
}

function posts(fetchMock: ReturnType<typeof mockApi>) {
	return fetchMock.mock.calls
		.filter(([, init]) => init?.method === 'POST')
		.map(([url, init]) => ({
			path: new URL(String(url)).pathname,
			body: JSON.parse(String(init?.body)),
		}))
}

describe('agent wizard', () => {
	beforeEach(() => {
		resetDemoState()
		resetAgentConfigs()
	})
	afterEach(() => {
		vi.unstubAllGlobals()
		clearAuth()
	})

	it('gates steps, offers twelve adapters and creates the agent through the API', async () => {
		const user = userEvent.setup()
		const created = teamAgent('a-darnold', 'Darnold', {
			slug: 'darnold',
			connector_grants: ['Chatwoot'],
		})
		const fetchMock = mockApi({
			'GET /api/v1/team/agents': { items: TEAM },
			'POST /api/v1/team/agents': { status: 201, body: created },
		})
		await renderAtPath(
			withQueryClient(<AgentWizardPage />),
			'/agents/nouveau',
			EXTRA,
		)
		expect(await screen.findByText(/Données de démonstration/)).toBeTruthy()

		expect((next() as HTMLButtonElement).disabled).toBe(true)
		await user.type(screen.getByLabelText("Nom de l'agent"), 'Darnold')
		await user.click(next())

		expect((next() as HTMLButtonElement).disabled).toBe(true)
		expect(screen.getAllByRole('button', { pressed: false }).length).toBe(12)
		await user.click(screen.getByRole('button', { name: 'Codex' }))
		await user.click(next())

		await user.click(screen.getByRole('button', { name: /Clé API/ }))
		expect((next() as HTMLButtonElement).disabled).toBe(true)
		await user.type(screen.getByLabelText('Clé API'), 'sk-test')
		await user.click(next())

		expect(
			(screen.getByRole('checkbox', { name: 'Chatwoot' }) as HTMLInputElement)
				.checked,
		).toBe(false)
		expect(screen.queryByRole('checkbox', { name: 'E-mail' })).toBeNull()
		await user.type(screen.getByLabelText('Titre'), 'Relais')
		await user.selectOptions(
			await screen.findByLabelText('Rapporte à'),
			'a-chief',
		)
		await user.click(screen.getByRole('checkbox', { name: 'Chatwoot' }))
		await user.click(
			screen.getByRole('checkbox', { name: /Pièces reçues sur Chatwoot/ }),
		)
		await user.click(next())

		expect(screen.getAllByText('Darnold')).toHaveLength(2)
		expect(screen.getByText('Chief of Staff')).toBeTruthy()
		expect(screen.queryByText('Aucun (accès refusé)')).toBeNull()
		await user.click(screen.getByRole('button', { name: "Tester l'agent" }))
		expect(screen.getByText(/Test réussi/)).toBeTruthy()
		await user.click(screen.getByRole('button', { name: "Créer l'agent" }))

		await waitFor(() => expect(posts(fetchMock)).toHaveLength(1))
		const [post] = posts(fetchMock)
		expect(post?.path).toBe('/api/v1/team/agents')
		expect(post?.body).toMatchObject({
			slug: 'darnold',
			name: 'Darnold',
			description: 'Relais',
			reports_to: 'a-chief',
			connector_grants: ['Chatwoot'],
		})
		await waitFor(() =>
			expect(
				demoStore.state.sources.find(
					(s) => s.id === 'pieces-recues-sur-chatwoot',
				)?.agentIds,
			).toContain('a-darnold'),
		)
		expect(getAgentConfig('a-darnold').fields.adapter).toBe('Codex')
	}, 40000)

	it('derives a free slug from the name', async () => {
		const user = userEvent.setup()
		const fetchMock = mockApi({
			'GET /api/v1/team/agents': {
				items: [teamAgent('a-dev', 'Dev', { slug: 'dev' })],
			},
			'POST /api/v1/team/agents': {
				status: 201,
				body: teamAgent('a-dev-2', 'Dev'),
			},
		})
		await renderAtPath(
			withQueryClient(<AgentWizardPage />),
			'/agents/nouveau',
			EXTRA,
		)
		await screen.findByLabelText("Nom de l'agent")
		await waitFor(() =>
			expect(
				fetchMock.mock.calls.some(([url]) =>
					String(url).endsWith('/api/v1/team/agents?archived=false'),
				),
			).toBe(true),
		)
		await toLastStep(user, 'Dev')
		await user.click(screen.getByRole('button', { name: "Créer l'agent" }))
		await waitFor(() => expect(posts(fetchMock)).toHaveLength(1))
		expect(posts(fetchMock)[0]?.body.slug).toBe('dev-2')
	})

	it('explains a duplicate name and stays on the wizard', async () => {
		const user = userEvent.setup()
		mockApi({
			'GET /api/v1/team/agents': { items: [] },
			'POST /api/v1/team/agents': {
				status: 409,
				body: { code: 'conflict', message: 'dup', status: 409 },
			},
		})
		await renderAtPath(
			withQueryClient(<AgentWizardPage />),
			'/agents/nouveau',
			EXTRA,
		)
		await toLastStep(user, 'Dev')
		await user.click(screen.getByRole('button', { name: "Créer l'agent" }))
		expect(await screen.findByText('Un agent porte déjà ce nom.')).toBeTruthy()
		expect(screen.getByRole('button', { name: "Créer l'agent" })).toBeTruthy()
	})

	it('goes back and keeps the answers', async () => {
		const user = userEvent.setup()
		mockApi({ 'GET /api/v1/team/agents': { items: [] } })
		await renderAtPath(
			withQueryClient(<AgentWizardPage />),
			'/agents/nouveau',
			EXTRA,
		)
		await user.type(await screen.findByLabelText("Nom de l'agent"), 'Zoé')
		await user.click(next())
		await user.click(screen.getByRole('button', { name: '← Retour' }))
		expect(
			(screen.getByLabelText("Nom de l'agent") as HTMLInputElement).value,
		).toBe('Zoé')
	})

	it('creates with no connector and no parent by default', async () => {
		const user = userEvent.setup()
		const fetchMock = mockApi({
			'GET /api/v1/team/agents': { items: [] },
			'POST /api/v1/team/agents': {
				status: 201,
				body: teamAgent('a-nu', 'Nu'),
			},
		})
		await renderAtPath(
			withQueryClient(<AgentWizardPage />),
			'/agents/nouveau',
			EXTRA,
		)
		await toLastStep(user, 'Nu')
		expect(screen.getByText('Aucun (accès refusé)')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: "Créer l'agent" }))
		await waitFor(() => expect(posts(fetchMock)).toHaveLength(1))
		expect(posts(fetchMock)[0]?.body).toMatchObject({
			slug: 'nu',
			connector_grants: [],
			reports_to: null,
		})
	})
})
