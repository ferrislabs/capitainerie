import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { AgentWizardPage } from '#/domain/agent-team/feature/agent-wizard-page'
import { skill } from '#/domain/skills/test-fixtures'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { HARNESS_ROUTES, personalKey } from '#/test-utils/harnesses'
import { TEAM, teamAgent } from '#/test-utils/team-agents'

const EXTRA = ['/agents', '/agents/nouveau', '/agents/$agentId/{-$tab}']

const next = () => screen.getByRole('button', { name: /→$/ })

async function toLastStep(
	user: ReturnType<typeof userEvent.setup>,
	name: string,
) {
	await user.type(await screen.findByLabelText("Nom de l'agent"), name)
	await user.click(next())
	await user.click(await screen.findByRole('button', { name: 'Codex' }))
	await user.click(next())
	await user.click(next())
	await user.click(next())
}

const CONNECTORS = {
	...HARNESS_ROUTES,
	'GET /api/v1/skills': {
		items: [skill('humanizer'), skill('gitlab-mr')],
	},
	'GET /api/v1/mcp-connectors': {
		items: [
			{
				id: 'c-1',
				slug: 'chatwoot',
				name: 'Chatwoot',
				url: 'https://chat.example.com/mcp',
				issuer: 'https://chat.example.com',
				resource: 'https://chat.example.com/mcp',
				scopes: [],
				connected: true,
				archived: false,
				created_by: 'u',
				created_at: '2026-11-01T00:00:00Z',
				updated_at: '2026-11-01T00:00:00Z',
			},
		],
	},
}

function calls(fetchMock: ReturnType<typeof mockApi>, method: string) {
	return fetchMock.mock.calls
		.filter(([, init]) => init?.method === method)
		.map(([url, init]) => ({
			path: new URL(String(url)).pathname,
			body: JSON.parse(String(init?.body)),
		}))
}

const posts = (fetchMock: ReturnType<typeof mockApi>) =>
	calls(fetchMock, 'POST')

describe('agent wizard', () => {
	afterEach(() => {
		vi.unstubAllGlobals()
		clearAuth()
	})

	it('gates steps, offers the four harnesses and creates the agent through the API', async () => {
		const user = userEvent.setup()
		const created = teamAgent('a-darnold', 'Darnold', {
			slug: 'darnold',
			connector_grants: ['chatwoot'],
		})
		const fetchMock = mockApi({
			...CONNECTORS,
			'GET /api/v1/team/agents': { items: TEAM },
			'POST /api/v1/team/agents': { status: 201, body: created },
			'PATCH /api/v1/team/agents/a-darnold': { ...created, revision: 2 },
		})
		await renderAtPath(
			withQueryClient(<AgentWizardPage />),
			'/agents/nouveau',
			EXTRA,
		)
		expect(screen.queryByText(/Données de démonstration/)).toBeNull()

		expect((next() as HTMLButtonElement).disabled).toBe(true)
		await user.type(await screen.findByLabelText("Nom de l'agent"), 'Darnold')
		await user.click(next())

		expect((next() as HTMLButtonElement).disabled).toBe(true)
		await screen.findByRole('button', { name: 'Codex' })
		expect(screen.getAllByRole('button', { pressed: false }).length).toBe(4)
		expect(screen.getByText('Mistral')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Codex' }))
		await user.click(next())

		expect(await screen.findByText('Clé OpenAI')).toBeTruthy()
		expect(screen.getByText(/Ta clé personnelle \(sk-…abcd\)/)).toBeTruthy()
		expect(screen.queryByText(/Claude/)).toBeNull()
		expect(screen.queryByLabelText('Clé API OpenAI')).toBeNull()
		await user.click(next())

		const models = within(screen.getByLabelText('Modèle')).getAllByRole(
			'option',
		)
		expect(models.map((o) => o.textContent)).toEqual([
			'gpt-6-sol',
			'gpt-6-mini',
		])
		await user.selectOptions(screen.getByLabelText('Modèle'), 'gpt-6-mini')

		expect(
			(
				(await screen.findByRole('checkbox', {
					name: 'Chatwoot',
				})) as HTMLInputElement
			).checked,
		).toBe(false)
		expect(screen.queryByText(/Sources à écouter/)).toBeNull()
		await user.type(screen.getByLabelText('Titre'), 'Relais')
		await user.selectOptions(
			await screen.findByLabelText('Rapporte à'),
			'a-chief',
		)
		await user.click(screen.getByRole('checkbox', { name: 'Chatwoot' }))
		await user.click(screen.getByRole('checkbox', { name: 'humanizer' }))
		await user.click(next())

		expect(screen.getAllByText('Darnold')).toHaveLength(2)
		expect(screen.getByText('gpt-6-mini · effort auto')).toBeTruthy()
		expect(screen.getByText(/Ta clé personnelle/)).toBeTruthy()
		expect(screen.getByText('Chief of Staff')).toBeTruthy()
		expect(screen.getByText('Chatwoot')).toBeTruthy()
		expect(screen.queryByRole('button', { name: "Tester l'agent" })).toBeNull()
		await user.click(screen.getByRole('button', { name: "Créer l'agent" }))

		await waitFor(() => expect(posts(fetchMock)).toHaveLength(1))
		const [post] = posts(fetchMock)
		expect(post?.path).toBe('/api/v1/team/agents')
		expect(post?.body).toMatchObject({
			slug: 'darnold',
			name: 'Darnold',
			description: 'Relais',
			reports_to: 'a-chief',
			connector_grants: ['chatwoot'],
			model: 'gpt-6-mini',
		})
		expect(calls(fetchMock, 'PUT')).toEqual([])
		await waitFor(() => expect(calls(fetchMock, 'PATCH')).toHaveLength(1))
		const [configured] = calls(fetchMock, 'PATCH')
		expect(configured?.path).toBe('/api/v1/team/agents/a-darnold')
		expect(configured?.body.config).toMatchObject({
			adapter: 'Codex',
			title: 'Relais',
		})
		expect(configured?.body.skills).toEqual(['humanizer'])
		expect(configured?.body.change_summary).toBe('configuration initiale')
	}, 40000)

	it('asks for the key when none exists and saves it as a personal key', async () => {
		const user = userEvent.setup()
		const created = teamAgent('a-zoe', 'Zoé', { slug: 'zoe' })
		const fetchMock = mockApi({
			...CONNECTORS,
			'GET /api/v1/me/providers': [],
			'GET /api/v1/team/agents': { items: [] },
			'PUT /api/v1/me/providers/openai': personalKey('openai'),
			'POST /api/v1/team/agents': { status: 201, body: created },
			'PATCH /api/v1/team/agents/a-zoe': created,
		})
		await renderAtPath(
			withQueryClient(<AgentWizardPage />),
			'/agents/nouveau',
			EXTRA,
		)
		await user.type(await screen.findByLabelText("Nom de l'agent"), 'Zoé')
		await user.click(next())
		await user.click(await screen.findByRole('button', { name: 'Codex' }))
		await user.click(next())
		expect(await screen.findByText('Clé OpenAI')).toBeTruthy()
		expect(screen.queryByText(/Claude/)).toBeNull()
		expect(screen.queryByText(/Abonnement/)).toBeNull()
		await waitFor(() =>
			expect(screen.getByLabelText('Clé API OpenAI')).toBeTruthy(),
		)
		expect((next() as HTMLButtonElement).disabled).toBe(true)
		await user.type(screen.getByLabelText('Clé API OpenAI'), 'sk-test')
		await user.click(next())
		await user.click(next())
		expect(screen.getByText('Ta nouvelle clé personnelle')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: "Créer l'agent" }))
		await waitFor(() => expect(calls(fetchMock, 'POST')).toHaveLength(1))
		expect(calls(fetchMock, 'PUT')).toEqual([
			{
				path: '/api/v1/me/providers/openai',
				body: {
					api_key: 'sk-test',
					default_model: 'gpt-6-sol',
					is_default: false,
				},
			},
		])
	}, 40000)

	it('does not create the agent when saving the key fails', async () => {
		const user = userEvent.setup()
		const fetchMock = mockApi({
			...CONNECTORS,
			'GET /api/v1/me/providers': [],
			'GET /api/v1/providers': {
				status: 403,
				body: { code: 'forbidden', message: 'no', status: 403 },
			},
			'GET /api/v1/team/agents': { items: [] },
			'PUT /api/v1/me/providers/openai': {
				status: 422,
				body: { code: 'invalid', message: 'bad', status: 422 },
			},
		})
		await renderAtPath(
			withQueryClient(<AgentWizardPage />),
			'/agents/nouveau',
			EXTRA,
		)
		await user.type(await screen.findByLabelText("Nom de l'agent"), 'Zoé')
		await user.click(next())
		await user.click(await screen.findByRole('button', { name: 'Codex' }))
		await user.click(next())
		expect(await screen.findByText(/sera utilisée si elle existe/)).toBeTruthy()
		await user.type(screen.getByLabelText('Clé API OpenAI'), 'sk-bad')
		await user.click(next())
		await user.click(next())
		await user.click(screen.getByRole('button', { name: "Créer l'agent" }))
		expect(
			await screen.findByText(/Impossible d'enregistrer ta clé/),
		).toBeTruthy()
		expect(calls(fetchMock, 'POST')).toEqual([])
	}, 40000)

	it('uses the organization key without asking for one', async () => {
		const user = userEvent.setup()
		mockApi({
			...CONNECTORS,
			'GET /api/v1/me/providers': [],
			'GET /api/v1/providers': {
				providers: [
					{
						kind: 'anthropic',
						preview: 'sk-ant…',
						created_by: 'u',
						updated_at: '2026-11-01T00:00:00Z',
					},
				],
			},
			'GET /api/v1/team/agents': { items: [] },
		})
		await renderAtPath(
			withQueryClient(<AgentWizardPage />),
			'/agents/nouveau',
			EXTRA,
		)
		await user.type(await screen.findByLabelText("Nom de l'agent"), 'Zoé')
		await user.click(next())
		await user.click(await screen.findByRole('button', { name: 'Claude Code' }))
		await user.click(next())
		expect(await screen.findByText('Clé Anthropic')).toBeTruthy()
		expect(
			await screen.findByText(/clé de l'organisation sera utilisée/),
		).toBeTruthy()
		expect(screen.queryByLabelText('Clé API Anthropic')).toBeNull()
		expect((next() as HTMLButtonElement).disabled).toBe(false)
	})

	it('shows an error when harnesses cannot be loaded', async () => {
		const user = userEvent.setup()
		mockApi({
			'GET /api/v1/harnesses': {
				status: 500,
				body: { code: 'x', message: 'x', status: 500 },
			},
			'GET /api/v1/team/agents': { items: [] },
		})
		await renderAtPath(
			withQueryClient(<AgentWizardPage />),
			'/agents/nouveau',
			EXTRA,
		)
		await user.type(await screen.findByLabelText("Nom de l'agent"), 'Zoé')
		await user.click(next())
		expect(
			await screen.findByText('Impossible de charger les adaptateurs.'),
		).toBeTruthy()
	})

	it('derives a free slug from the name', async () => {
		const user = userEvent.setup()
		const fetchMock = mockApi({
			...HARNESS_ROUTES,
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
			...HARNESS_ROUTES,
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
			...HARNESS_ROUTES,
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
