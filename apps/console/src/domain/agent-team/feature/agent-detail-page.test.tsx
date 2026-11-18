import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { renderAtPath } from '#/components/layout/test-router'
import {
	getAgentConfig,
	resetAgentConfigs,
	seedAgentConfig,
} from '#/domain/agent-team/agent-config-store'
import { AGENT_TABS } from '#/domain/agent-team/agent-tabs'
import { AgentDetailPage } from '#/domain/agent-team/feature/agent-detail-page'
import { demoStore, resetDemoState } from '#/domain/demo'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import {
	DEV,
	LEGAL,
	REGISTRE,
	TEAM,
	teamAgent,
	teamRoutes,
} from '#/test-utils/team-agents'

const EXTRA = [
	'/connectors/{-$tab}',
	'/sources/$sourceId/{-$tab}',
	'/agents',
	'/agents/organigramme',
	'/agents/$agentId/{-$tab}',
	'/tasks',
	'/tasks/$taskId',
]

type Page = Parameters<typeof AgentDetailPage>[0]
type Routes = Parameters<typeof mockApi>[0]

async function open(props: Page, extra: Routes = {}, agent = DEV) {
	const fetchMock = mockApi({
		...teamRoutes(agent),
		[`PATCH /api/v1/team/agents/${agent.id}`]: agent,
		...extra,
	})
	await renderAtPath(
		withQueryClient(<AgentDetailPage {...props} />),
		`/agents/${props.agentId}`,
		EXTRA,
	)
	await screen.findByRole('navigation', { name: "Sections de l'agent" })
	return fetchMock
}

function patches(fetchMock: ReturnType<typeof mockApi>) {
	return fetchMock.mock.calls
		.filter(([, init]) => init?.method === 'PATCH')
		.map(([url, init]) => ({
			path: new URL(String(url)).pathname,
			body: JSON.parse(String(init?.body)),
		}))
}

describe('agent detail page', () => {
	beforeEach(() => {
		resetDemoState()
		resetAgentConfigs()
	})
	afterEach(() => {
		vi.unstubAllGlobals()
		clearAuth()
	})

	it('renders header and eleven tab links with the active one marked', async () => {
		await open({ agentId: 'a-dev', tab: 'skills' })
		expect(
			screen.getByRole('heading', { name: /Dev \(cycle autonome\)/ }),
		).toBeTruthy()
		const nav = screen.getByRole('navigation', { name: "Sections de l'agent" })
		for (const tab of AGENT_TABS)
			expect(within(nav).getByRole('link', { name: tab.label })).toBeTruthy()
		expect(
			within(nav)
				.getByRole('link', { name: 'Compétences' })
				.getAttribute('aria-current'),
		).toBe('page')
		expect(
			within(nav)
				.getByRole('link', { name: 'Exécutions' })
				.getAttribute('href'),
		).toBe('/audit/runs')
		for (const label of [
			'Définir comme principal',
			'Assigner une tâche',
			'Lancer',
			'Pause',
			'Archiver',
		])
			expect(screen.getByRole('button', { name: label })).toBeTruthy()
	})

	it('falls back to overview and shows the real identity', async () => {
		await open({ agentId: 'a-dev', tab: 'bogus' })
		expect(screen.getByText('Harness / Runtime')).toBeTruthy()
		expect(screen.getByText('Description Dev (cycle autonome)')).toBeTruthy()
		expect(screen.getByText('Chief of Staff')).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: "Vue d'ensemble" })
				.getAttribute('aria-current'),
		).toBe('page')
	})

	it('opens connectors for the legacy tools segment', async () => {
		await open({ agentId: 'a-dev', tab: 'tools' })
		expect(screen.getByText("Connecteurs de l'agent")).toBeTruthy()
	})

	it('toggles pause and shows a message when launching', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'a-dev' })
		await user.click(screen.getByRole('button', { name: 'Pause' }))
		expect(screen.getByRole('button', { name: 'Reprendre' })).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Lancer' }))
		expect(screen.getByText('Exécution lancée pour Dev')).toBeTruthy()
	})

	it('shows a loading state, then not-found on 404', async () => {
		mockApi({
			'GET /api/v1/team/agents': { items: TEAM },
			'GET /api/v1/team/agents/nope': {
				status: 404,
				body: { code: 'not_found', message: 'nope', status: 404 },
			},
		})
		await renderAtPath(
			withQueryClient(<AgentDetailPage agentId="nope" />),
			'/agents/nope',
			EXTRA,
		)
		expect(screen.getByText('Chargement…')).toBeTruthy()
		expect(await screen.findByText('Agent introuvable')).toBeTruthy()
	})

	it('shows an error state when loading fails', async () => {
		mockApi({
			'GET /api/v1/team/agents': { items: TEAM },
			'GET /api/v1/team/agents/a-dev': {
				status: 500,
				body: { code: 'internal', message: 'boom', status: 500 },
			},
		})
		await renderAtPath(
			withQueryClient(<AgentDetailPage agentId="a-dev" />),
			'/agents/a-dev',
			EXTRA,
		)
		expect(
			await screen.findByText("Impossible de charger l'agent."),
		).toBeTruthy()
	})

	describe('archive', () => {
		it('archives the agent', async () => {
			const user = userEvent.setup()
			const fetchMock = await open({ agentId: 'a-dev' })
			await user.click(screen.getByRole('button', { name: 'Archiver' }))
			expect(await screen.findByText('Dev archivé')).toBeTruthy()
			expect(patches(fetchMock)).toEqual([
				{ path: '/api/v1/team/agents/a-dev', body: { archived: true } },
			])
		})

		it('restores an archived agent', async () => {
			const user = userEvent.setup()
			const archived = teamAgent('a-dev', 'Dev (cycle autonome)', {
				archived: true,
			})
			const fetchMock = await open({ agentId: 'a-dev' }, {}, archived)
			expect(screen.getByText('Archivé')).toBeTruthy()
			await user.click(screen.getByRole('button', { name: 'Restaurer' }))
			expect(await screen.findByText('Dev restauré')).toBeTruthy()
			expect(patches(fetchMock)[0]?.body).toEqual({ archived: false })
		})
	})

	describe('instructions', () => {
		it('reads the instructions of the agent', async () => {
			await open({ agentId: 'a-dev', tab: 'instructions', mode: 'lire' })
			expect(screen.getByText(/^# Dev · Natalia/)).toBeTruthy()
			expect(
				screen.getByRole('link', { name: 'Éditer' }).getAttribute('href'),
			).toContain('mode=editer')
		})

		it('saves an edit through the API and lists it as a revision', async () => {
			const user = userEvent.setup()
			const fetchMock = await open({
				agentId: 'a-dev',
				tab: 'instructions',
				mode: 'editer',
			})
			expect(screen.queryByText('Modifications non enregistrées')).toBeNull()
			await user.type(screen.getByLabelText('Instructions'), ' plus')
			expect(screen.getByText('Modifications non enregistrées')).toBeTruthy()
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(
				await screen.findByText('Enregistré · révision 1 créée'),
			).toBeTruthy()
			expect(patches(fetchMock)).toEqual([
				{
					path: '/api/v1/team/agents/a-dev',
					body: { instructions: '# Dev · Natalia plus' },
				},
			])
			expect(getAgentConfig('a-dev').revs[0]?.changed).toBe('instructions')
		})

		it('cancels an edit without calling the API', async () => {
			const user = userEvent.setup()
			const fetchMock = await open({
				agentId: 'a-dev',
				tab: 'instructions',
				mode: 'editer',
			})
			await user.type(screen.getByLabelText('Instructions'), 'x')
			await user.click(screen.getByRole('button', { name: 'Annuler' }))
			expect(screen.queryByText('Modifications non enregistrées')).toBeNull()
			expect(patches(fetchMock)).toEqual([])
		})

		it('keeps the draft and explains a rejected save', async () => {
			const user = userEvent.setup()
			await open(
				{ agentId: 'a-dev', tab: 'instructions', mode: 'editer' },
				{
					'PATCH /api/v1/team/agents/a-dev': {
						status: 403,
						body: { code: 'forbidden', message: 'no', status: 403 },
					},
				},
			)
			await user.type(screen.getByLabelText('Instructions'), 'x')
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(
				await screen.findByText("Tu n'as pas le droit de modifier les agents."),
			).toBeTruthy()
			expect(screen.getByText('Modifications non enregistrées')).toBeTruthy()
		})
	})

	it('shows the empty revisions state, then local revisions', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'a-dev', tab: 'rev' })
		expect(screen.getByText('Aucune révision')).toBeTruthy()
		seedAgentConfig('a-dev', {
			...getAgentConfig('a-dev'),
			rev: 2,
			revs: [
				{ label: 'Rév. 2', author: 'A', when: 'x', changed: 'modèle' },
				{ label: 'Rév. 1', author: 'A', when: 'y', changed: 'nom' },
			],
		})
		expect(await screen.findByText('Actuelle')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Restaurer Rév. 1' }))
		expect(screen.getByText(/Rév. 1 restaurée/)).toBeTruthy()
	})

	describe('runtime', () => {
		it('patches identity, role, model and hierarchy', async () => {
			const user = userEvent.setup()
			const fetchMock = await open({ agentId: 'a-dev', tab: 'runtime' })
			await screen.findByRole('option', {
				name: 'Veille légale (Sous-domaines)',
			})
			await user.clear(screen.getByLabelText('Nom'))
			await user.type(screen.getByLabelText('Nom'), 'Dev 2')
			await user.clear(screen.getByLabelText('Description'))
			await user.type(screen.getByLabelText('Description'), 'Nouvelle')
			await user.selectOptions(screen.getByLabelText('Rôle'), 'DevOps')
			await user.clear(screen.getByLabelText('Modèle'))
			await user.type(screen.getByLabelText('Modèle'), 'claude-haiku-5-5')
			await user.selectOptions(screen.getByLabelText('Rapporte à'), 'a-legal')
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(await screen.findByText(/Enregistré · révision 1/)).toBeTruthy()
			expect(patches(fetchMock)[0]?.body).toEqual({
				name: 'Dev 2',
				description: 'Nouvelle',
				role: 'DevOps',
				model: 'claude-haiku-5-5',
				reports_to: 'a-legal',
			})
		})

		it('hints at the providers screen and rejects display names', async () => {
			const user = userEvent.setup()
			const fetchMock = await open({ agentId: 'a-dev', tab: 'runtime' })
			expect(
				screen.getByText(/Le modèle utilisé est celui de votre fournisseur/),
			).toBeTruthy()
			expect(
				screen
					.getByRole('link', { name: 'Gérer mes fournisseurs' })
					.getAttribute('href'),
			).toBe('/providers')
			const model = screen.getByLabelText('Modèle')
			expect(model.getAttribute('placeholder')).toBe(
				'ex. claude-sonnet-5-5 (vide = modèle par défaut du fournisseur)',
			)
			await user.clear(model)
			await user.type(model, 'Claude Haiku 5.5')
			expect(screen.getByText(/Identifiant de modèle invalide/)).toBeTruthy()
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(patches(fetchMock)).toEqual([])
			await user.clear(model)
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(await screen.findByText(/Enregistré · révision 1/)).toBeTruthy()
			expect(patches(fetchMock)[0]?.body).toEqual({ model: '' })
		})

		it('moves an agent back to the direction with a null parent', async () => {
			const user = userEvent.setup()
			const fetchMock = await open({ agentId: 'a-dev', tab: 'runtime' })
			await user.selectOptions(
				screen.getByLabelText('Rapporte à'),
				'Aucun (direction)',
			)
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			await waitFor(() => expect(patches(fetchMock)).toHaveLength(1))
			expect(patches(fetchMock)[0]?.body).toEqual({ reports_to: null })
		})

		it('explains a hierarchy cycle rejected by the API', async () => {
			const user = userEvent.setup()
			await open(
				{ agentId: 'a-chief', tab: 'runtime' },
				{
					...teamRoutes(teamAgent('a-chief', 'Chief of Staff')),
					'PATCH /api/v1/team/agents/a-chief': {
						status: 422,
						body: { code: 'cycle', message: 'cycle', status: 422 },
					},
				},
				teamAgent('a-chief', 'Chief of Staff'),
			)
			await user.selectOptions(screen.getByLabelText('Rapporte à'), 'a-dev')
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(await screen.findByText(/ne peut pas dépendre/)).toBeTruthy()
			expect(screen.getByText('Modifications non enregistrées')).toBeTruthy()
		})

		it('keeps local-only changes off the network', async () => {
			const user = userEvent.setup()
			const fetchMock = await open({ agentId: 'a-dev', tab: 'runtime' })
			await user.selectOptions(
				screen.getByLabelText('Effort de réflexion'),
				'Élevé',
			)
			await user.click(screen.getByRole('switch', { name: 'Activer Chrome' }))
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(getAgentConfig('a-dev').revs[0]?.changed).toBe(
				'effort de réflexion, Chrome',
			)
			expect(screen.getByText('Enregistré · révision 1 créée')).toBeTruthy()
			expect(patches(fetchMock)).toEqual([])
			await user.click(screen.getByRole('button', { name: 'Lancer le test' }))
			expect(screen.getByText(/Test réussi/)).toBeTruthy()
		})
	})

	it('permissions tab saves locally with a revision', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'a-dev', tab: 'perms' })
		await user.click(screen.getByRole('switch', { name: 'Créer des agents' }))
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(getAgentConfig('a-dev').fields.pCreate).toBe(true)
		expect(getAgentConfig('a-dev').revs[0]?.changed).toBe(
			'droit de créer des agents',
		)
	})

	it('skills: switches, count and autosave message', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'a-dev', tab: 'skills' })
		expect(screen.getByText('8 sur 86')).toBeTruthy()
		await user.click(screen.getByRole('switch', { name: 'Activer ui-ux' }))
		expect(screen.getByText('9 sur 86')).toBeTruthy()
		expect(screen.getByText('ui-ux activée')).toBeTruthy()
	})

	it('sources: attach and detach through the demo store, keyed by uuid', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'a-legal', tab: 'sources' }, {}, LEGAL)
		expect(screen.getByText("Cet agent n'écoute aucune source")).toBeTruthy()
		await user.selectOptions(
			screen.getByLabelText('Source'),
			'nouveaux-sous-domaines-sensibles',
		)
		await user.click(screen.getByRole('button', { name: 'Écouter' }))
		const source = () =>
			demoStore.state.sources.find(
				(s) => s.id === 'nouveaux-sous-domaines-sensibles',
			)
		expect(source()?.agentIds).toContain('a-legal')
		await user.click(screen.getByRole('button', { name: /Arrêter d'écouter/ }))
		expect(source()?.agentIds).not.toContain('a-legal')
	})

	describe('delegation', () => {
		it('shows the parent and the sub-agents from reports_to', async () => {
			await open({ agentId: 'a-legal', tab: 'team' }, {}, LEGAL)
			expect(
				(await screen.findAllByRole('link', { name: /Registre/ })).length,
			).toBeGreaterThan(0)
			expect(
				screen.getAllByRole('link', { name: /Chief of Staff/ }).length,
			).toBeGreaterThan(0)
			expect(screen.getAllByText(/ouverte/).length).toBeGreaterThan(0)
			expect(screen.getByText(/n'hérite d'aucun connecteur/)).toBeTruthy()
		})

		it('removes a sub-agent back to the parent of this agent', async () => {
			const user = userEvent.setup()
			const fetchMock = await open(
				{ agentId: 'a-legal', tab: 'team' },
				{ 'PATCH /api/v1/team/agents/a-registre': REGISTRE },
				LEGAL,
			)
			await user.click(
				await screen.findByRole('button', { name: /Retirer Registre/ }),
			)
			await waitFor(() => expect(patches(fetchMock)).toHaveLength(1))
			expect(patches(fetchMock)[0]).toEqual({
				path: '/api/v1/team/agents/a-registre',
				body: { reports_to: 'a-chief' },
			})
		})

		it('adds a sub-agent but never offers self or ancestors', async () => {
			const user = userEvent.setup()
			const fetchMock = await open(
				{ agentId: 'a-legal', tab: 'team' },
				{ 'PATCH /api/v1/team/agents/a-dev': DEV },
				LEGAL,
			)
			const select = await screen.findByLabelText('Agent à ajouter')
			const names = within(select)
				.getAllByRole('option')
				.map((o) => o.textContent)
			expect(names).not.toContain('Veille légale (Sous-domaines)')
			expect(names).not.toContain('Chief of Staff')
			await user.selectOptions(select, 'a-dev')
			await user.click(screen.getByRole('button', { name: 'Ajouter' }))
			await waitFor(() => expect(patches(fetchMock)).toHaveLength(1))
			expect(patches(fetchMock)[0]).toEqual({
				path: '/api/v1/team/agents/a-dev',
				body: { reports_to: 'a-legal' },
			})
		})

		it('reports a cycle refused by the API', async () => {
			const user = userEvent.setup()
			await open(
				{ agentId: 'a-legal', tab: 'team' },
				{
					'PATCH /api/v1/team/agents/a-dev': {
						status: 422,
						body: { code: 'cycle', message: 'cycle', status: 422 },
					},
				},
				LEGAL,
			)
			await user.selectOptions(
				await screen.findByLabelText('Agent à ajouter'),
				'a-dev',
			)
			await user.click(screen.getByRole('button', { name: 'Ajouter' }))
			expect(await screen.findByText(/ne peut pas dépendre/)).toBeTruthy()
		})

		it('rules form saves with a local revision', async () => {
			const user = userEvent.setup()
			await open({ agentId: 'a-dev', tab: 'team' })
			await user.click(screen.getByRole('switch', { name: 'Peut déléguer' }))
			await user.selectOptions(
				screen.getByLabelText('Profondeur maximale'),
				'3',
			)
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(getAgentConfig('a-dev').revs[0]?.changed).toBe(
				'droit de déléguer, profondeur de délégation',
			)
		})
	})

	describe('connectors', () => {
		const connector = (
			slug: string,
			name: string,
			connected: boolean,
		): Schemas.McpConnectorResponse => ({
			id: slug,
			slug,
			name,
			url: `https://${slug}.example.com/mcp`,
			resource: `https://${slug}.example.com/mcp`,
			issuer: `https://${slug}.example.com`,
			scopes: [],
			archived: false,
			connected,
			expires_at: null,
			created_by: 'u',
			created_at: '2026-11-01T00:00:00Z',
			updated_at: '2026-11-01T00:00:00Z',
		})
		const MCP = {
			'GET /api/v1/mcp-connectors': {
				items: [
					connector('mestier', 'Mestier', true),
					connector('slack', 'Slack', false),
				],
			},
		}

		it('is deny by default and grants through the API', async () => {
			const user = userEvent.setup()
			const fetchMock = await open({ agentId: 'a-dev', tab: 'connectors' }, MCP)
			const mestier = await screen.findByRole('switch', {
				name: 'Autoriser Mestier',
			})
			expect(mestier.getAttribute('aria-checked')).toBe('false')
			await user.click(mestier)
			expect(await screen.findByText('Dev peut utiliser Mestier')).toBeTruthy()
			expect(patches(fetchMock)[0]?.body).toEqual({
				connector_grants: ['mestier'],
			})
		})

		it('shows and revokes existing grants by slug', async () => {
			const user = userEvent.setup()
			const granted = teamAgent('a-dev', 'Dev (cycle autonome)', {
				connector_grants: ['mestier'],
			})
			const fetchMock = await open(
				{ agentId: 'a-dev', tab: 'connectors' },
				MCP,
				granted,
			)
			const toggle = await screen.findByRole('switch', {
				name: 'Autoriser Mestier',
			})
			expect(toggle.getAttribute('aria-checked')).toBe('true')
			await user.click(toggle)
			expect(await screen.findByText('Accès retiré à Dev')).toBeTruthy()
			expect(patches(fetchMock)[0]?.body).toEqual({ connector_grants: [] })
		})

		it('flags a connector the user is not connected to', async () => {
			const user = userEvent.setup()
			const fetchMock = await open({ agentId: 'a-dev', tab: 'connectors' }, MCP)
			await user.click(
				await screen.findByRole('switch', { name: 'Autoriser Slack' }),
			)
			expect(await screen.findByText('Dev peut utiliser Slack')).toBeTruthy()
			expect(patches(fetchMock)[0]?.body).toEqual({
				connector_grants: ['slack'],
			})
			expect(screen.getAllByText('Non connecté').length).toBeGreaterThan(0)
			expect(screen.getByLabelText('Compte GitHub utilisé')).toBeTruthy()
		})

		it('shows an empty state without connectors', async () => {
			await open(
				{ agentId: 'a-dev', tab: 'connectors' },
				{ 'GET /api/v1/mcp-connectors': { items: [] } },
			)
			expect(await screen.findByText('Aucun connecteur')).toBeTruthy()
		})
	})

	it('has no fake keys or secrets tabs', async () => {
		await open({ agentId: 'a-dev', tab: 'keys' })
		expect(screen.queryByRole('link', { name: /Clés API/ })).toBeNull()
		expect(screen.queryByRole('link', { name: /Secrets/ })).toBeNull()
		expect(screen.queryByRole('button', { name: 'Créer une clé' })).toBeNull()
	})
})
