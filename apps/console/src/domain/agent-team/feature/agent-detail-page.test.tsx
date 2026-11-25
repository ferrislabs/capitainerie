import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { renderAtPath } from '#/components/layout/test-router'
import { AGENT_TABS } from '#/domain/agent-team/agent-tabs'
import { AgentDetailPage } from '#/domain/agent-team/feature/agent-detail-page'
import { skill } from '#/domain/skills/test-fixtures'
import { source } from '#/domain/sources/test-fixtures'
import { card } from '#/domain/tasks/test-fixtures'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { HARNESS_LIST } from '#/test-utils/harnesses'
import {
	DEV,
	identityOf,
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
		[`PATCH /api/v1/team/agents/${agent.id}`]: { ...agent, revision: 2 },
		'GET /api/v1/harnesses': HARNESS_LIST,
		'GET /api/v1/cards': { items: [] },
		'GET /api/v1/runs/active': { items: [] },
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

function patches(fetchMock: ReturnType<typeof mockApi>, method = 'PATCH') {
	return fetchMock.mock.calls
		.filter(([, init]) => init?.method === method)
		.map(([url, init]) => ({
			path: new URL(String(url)).pathname,
			body: init?.body ? JSON.parse(String(init.body)) : undefined,
		}))
}

describe('agent detail page', () => {
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
			within(nav).getByRole('link', { name: 'Audit →' }).getAttribute('href'),
		).toBe('/audit/runs')
		for (const label of ['Assigner une tâche', 'Pause', "Plus d'actions"])
			expect(screen.getByRole('button', { name: label })).toBeTruthy()
		for (const label of ['Définir comme principal', 'Lancer'])
			expect(screen.queryByRole('button', { name: label })).toBeNull()
		expect(screen.queryByText(/Données de démonstration/)).toBeNull()
	})

	it('falls back to overview and shows the real identity', async () => {
		await open({ agentId: 'a-dev', tab: 'bogus' })
		expect(screen.getByText('Harness / Runtime')).toBeTruthy()
		expect(screen.queryByText('Clé publique')).toBeNull()
		expect(screen.getByText('Description Dev (cycle autonome)')).toBeTruthy()
		expect(screen.getByText('Chief of Staff')).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: "Vue d'ensemble" })
				.getAttribute('aria-current'),
		).toBe('page')
	})

	it("shows the agent's FerrisKey client and its status", async () => {
		await open({ agentId: 'a-dev' })
		const section = (await screen.findByText('Identité FerrisKey')).closest(
			'section',
		) as HTMLElement
		expect(
			await within(section).findByText('capitainerie-agent-a-dev'),
		).toBeTruthy()
		expect(within(section).getByText('active')).toBeTruthy()
		expect(
			within(section).getByText("L'agent agit avec son propre client."),
		).toBeTruthy()
	})

	it('shows why a pending client is not created yet', async () => {
		await open(
			{ agentId: 'a-dev' },
			{
				'GET /api/v1/team/agents/a-dev/identity': identityOf(DEV, {
					status: 'pending',
					failure: 'agent identity: FerrisKey answered 503',
				}),
			},
		)
		expect(await screen.findByText('en attente')).toBeTruthy()
		expect(
			screen.getByText('agent identity: FerrisKey answered 503'),
		).toBeTruthy()
	})

	it('opens connectors for the legacy tools segment', async () => {
		await open({ agentId: 'a-dev', tab: 'tools' })
		expect(screen.getByText("Connecteurs de l'agent")).toBeTruthy()
	})

	it('lists the cards assigned to the agent and its active run', async () => {
		const own = card('c0ffee00-0000-4000-8000-000000000001', {
			title: 'Corriger le build',
			column: 'in_progress',
			assignee_agent_id: 'a-dev',
		})
		const fetchMock = await open(
			{ agentId: 'a-dev' },
			{
				'GET /api/v1/cards': { items: [own] },
				'GET /api/v1/runs/active': {
					items: [
						{
							cost_micros: 0,
							input_tokens: 0,
							output_tokens: 0,
							steps_done: 1,
							steps_total: 3,
							run: {
								id: 'r-1',
								card_id: own.id,
								title: 'Run du build',
								created_at: new Date().toISOString(),
								initiated_by: 'u',
								pipeline: [],
								status: 'running',
							},
						},
					],
				},
			},
		)
		expect(await screen.findByText('Corriger le build')).toBeTruthy()
		expect(await screen.findByText(/Run du build · étape 2\/3/)).toBeTruthy()
		expect(
			screen.getByRole('heading', { name: 'Dernière exécution' }),
		).toBeTruthy()
		expect(screen.getByText('Étape 2 sur 3')).toBeTruthy()
		expect(
			fetchMock.mock.calls.some(([url]) =>
				String(url).includes('assignee_agent_id=a-dev'),
			),
		).toBe(true)
	})

	it('counts the tasks beyond the six most recent', async () => {
		const items = Array.from({ length: 8 }, (_, i) =>
			card(`c0ffee00-0000-4000-8000-00000000001${i}`, {
				title: `Tâche ${i}`,
				assignee_agent_id: 'a-dev',
			}),
		)
		await open({ agentId: 'a-dev' }, { 'GET /api/v1/cards': { items } })
		expect(await screen.findByText('+2 autres tâches')).toBeTruthy()
		expect(
			screen.queryByRole('heading', { name: 'Dernière exécution' }),
		).toBeNull()
	})

	it('shows an empty task list on the overview', async () => {
		await open({ agentId: 'a-dev' })
		expect(
			await screen.findByText('Aucune tâche assignée à cet agent.'),
		).toBeTruthy()
		expect(screen.getByText('Aucun run en cours')).toBeTruthy()
	})

	it('pauses and resumes through the API', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({ agentId: 'a-dev' })
		await user.click(screen.getByRole('button', { name: 'Pause' }))
		expect(await screen.findByText('Agent mis en pause')).toBeTruthy()
		expect(patches(fetchMock)[0]).toEqual({
			path: '/api/v1/team/agents/a-dev',
			body: { paused: true, change_summary: 'pause' },
		})
	})

	it('shows the resume button for a paused agent', async () => {
		const user = userEvent.setup()
		const paused = { ...DEV, paused: true }
		const fetchMock = await open({ agentId: 'a-dev' }, {}, paused)
		expect(screen.getByText('en pause')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Reprendre' }))
		await waitFor(() => expect(patches(fetchMock)).toHaveLength(1))
		expect(patches(fetchMock)[0]?.body).toEqual({
			paused: false,
			change_summary: 'reprise',
		})
	})

	it('opens the new task dialog with the agent preselected', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'a-dev' })
		await user.click(screen.getByRole('button', { name: 'Assigner une tâche' }))
		const dialog = await screen.findByRole('dialog')
		expect(within(dialog).getByText('Nouvelle tâche')).toBeTruthy()
		await waitFor(() =>
			expect(
				(within(dialog).getByLabelText('Assigner à') as HTMLSelectElement)
					.value,
			).toBe('a-dev'),
		)
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
			await user.click(screen.getByRole('button', { name: "Plus d'actions" }))
			await user.click(
				await screen.findByRole('menuitem', { name: 'Archiver' }),
			)
			expect(await screen.findByText('Dev archivé')).toBeTruthy()
			expect(patches(fetchMock)).toEqual([
				{
					path: '/api/v1/team/agents/a-dev',
					body: { archived: true, change_summary: 'archivage' },
				},
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
			expect(patches(fetchMock)[0]?.body).toEqual({
				archived: false,
				change_summary: 'restauration',
			})
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
				await screen.findByText('Enregistré · révision 2 créée'),
			).toBeTruthy()
			expect(patches(fetchMock)).toEqual([
				{
					path: '/api/v1/team/agents/a-dev',
					body: {
						instructions: '# Dev · Natalia plus',
						change_summary: 'instructions',
					},
				},
			])
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

	describe('revisions', () => {
		it('shows the empty revisions state', async () => {
			await open(
				{ agentId: 'a-dev', tab: 'rev' },
				{ 'GET /api/v1/team/agents/a-dev/revisions': { items: [] } },
			)
			expect(await screen.findByText('Aucune révision')).toBeTruthy()
		})

		it('lists revisions from the API and restores one', async () => {
			const user = userEvent.setup()
			const current = { ...DEV, revision: 2 }
			const fetchMock = await open(
				{ agentId: 'a-dev', tab: 'rev' },
				{
					'GET /api/v1/team/agents/a-dev/revisions': {
						items: [
							{
								revision: 2,
								changed: 'modèle',
								author: 'Baptiste',
								created_at: new Date().toISOString(),
							},
							{
								revision: 1,
								changed: 'création',
								author: 'Nathael',
								created_at: '2026-11-01T00:00:00Z',
							},
						],
					},
					'POST /api/v1/team/agents/a-dev/revisions/1/restore': {
						...current,
						revision: 3,
					},
				},
				current,
			)
			expect(await screen.findByText('Modifié : modèle')).toBeTruthy()
			expect(screen.getByText(/Baptiste · à l'instant/)).toBeTruthy()
			expect(screen.getByText('Actuelle')).toBeTruthy()
			await user.click(
				screen.getByRole('button', { name: 'Restaurer la révision 1' }),
			)
			expect(
				await screen.findByText('Révision 1 restaurée (révision 3 créée)'),
			).toBeTruthy()
			expect(patches(fetchMock, 'POST')[0]?.path).toBe(
				'/api/v1/team/agents/a-dev/revisions/1/restore',
			)
		})

		it('shows an error when revisions fail to load', async () => {
			await open(
				{ agentId: 'a-dev', tab: 'rev' },
				{
					'GET /api/v1/team/agents/a-dev/revisions': {
						status: 500,
						body: { code: 'internal', message: 'boom', status: 500 },
					},
				},
			)
			expect(
				await screen.findByText('Impossible de charger les révisions.'),
			).toBeTruthy()
		})
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
			await user.selectOptions(
				screen.getByLabelText('Modèle'),
				'claude-haiku-5-5',
			)
			await user.selectOptions(screen.getByLabelText('Rapporte à'), 'a-legal')
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(await screen.findByText(/Enregistré · révision 2/)).toBeTruthy()
			expect(patches(fetchMock)[0]?.body).toEqual({
				name: 'Dev 2',
				description: 'Nouvelle',
				role: 'DevOps',
				model: 'claude-haiku-5-5',
				reports_to: 'a-legal',
				change_summary: 'nom, description, rôle, hiérarchie, modèle',
			})
		})

		it('derives the adapter from the model and lists only its models', async () => {
			const user = userEvent.setup()
			const fetchMock = await open(
				{ agentId: 'a-dev', tab: 'runtime' },
				{},
				{ ...DEV, model: 'gpt-6-mini' },
			)
			await screen.findByRole('option', { name: 'Codex' })
			expect(
				(screen.getByLabelText('Adaptateur') as HTMLSelectElement).value,
			).toBe('Codex')
			expect(
				within(screen.getByLabelText('Modèle'))
					.getAllByRole('option')
					.map((o) => o.textContent),
			).toEqual(['gpt-6-sol', 'gpt-6-mini'])
			expect(
				screen.getByText(/L'adaptateur est décidé par le modèle/),
			).toBeTruthy()
			expect(
				screen
					.getByRole('link', { name: 'Gérer mes fournisseurs' })
					.getAttribute('href'),
			).toBe('/providers')
			await user.selectOptions(
				screen.getByLabelText('Adaptateur'),
				'Antigravity',
			)
			expect((screen.getByLabelText('Modèle') as HTMLSelectElement).value).toBe(
				'gemini-3-pro',
			)
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			await waitFor(() => expect(patches(fetchMock)).toHaveLength(1))
			expect(patches(fetchMock)[0]?.body).toMatchObject({
				model: 'gemini-3-pro',
				config: { adapter: 'Antigravity' },
			})
		})

		it('keeps an unknown current model as an extra first option', async () => {
			await open({ agentId: 'a-dev', tab: 'runtime' })
			await screen.findByRole('option', { name: 'claude-opus-5' })
			const options = within(screen.getByLabelText('Modèle')).getAllByRole(
				'option',
			)
			expect(options[0]?.textContent).toBe('Claude Sonnet 5.5')
			expect(options).toHaveLength(4)
			expect(
				(screen.getByLabelText('Adaptateur') as HTMLSelectElement).value,
			).toBe('Claude Code')
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
			expect(patches(fetchMock)[0]?.body).toEqual({
				reports_to: null,
				change_summary: 'hiérarchie',
			})
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

		it('stores runtime settings in the agent config', async () => {
			const user = userEvent.setup()
			const fetchMock = await open({ agentId: 'a-dev', tab: 'runtime' })
			expect(
				screen.queryByRole('button', { name: 'Lancer le test' }),
			).toBeNull()
			await user.selectOptions(
				screen.getByLabelText('Effort de réflexion'),
				'Élevé',
			)
			await user.click(screen.getByRole('switch', { name: 'Activer Chrome' }))
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(
				await screen.findByText('Enregistré · révision 2 créée'),
			).toBeTruthy()
			const [patch] = patches(fetchMock)
			expect(patch?.body.change_summary).toBe('effort de réflexion, Chrome')
			expect(patch?.body.config).toMatchObject({
				effort: 'Élevé',
				chrome: true,
			})
			expect(patch?.body).not.toHaveProperty('name')
		})

		it('reads runtime settings from the agent config', async () => {
			await open(
				{ agentId: 'a-dev', tab: 'runtime' },
				{},
				{ ...DEV, config: { effort: 'Moyen', chrome: 'oui' } },
			)
			expect(
				(screen.getByLabelText('Effort de réflexion') as HTMLSelectElement)
					.value,
			).toBe('Moyen')
			expect(
				screen
					.getByRole('switch', { name: 'Activer Chrome' })
					.getAttribute('aria-checked'),
			).toBe('false')
		})
	})

	it('permissions tab saves into the config', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({ agentId: 'a-dev', tab: 'perms' })
		await user.click(screen.getByRole('switch', { name: 'Créer des agents' }))
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		await waitFor(() => expect(patches(fetchMock)).toHaveLength(1))
		expect(patches(fetchMock)[0]?.body).toMatchObject({
			config: { pCreate: true },
			change_summary: 'droit de créer des agents',
		})
	})

	describe('skills', () => {
		it('lists the agent skills and removes one', async () => {
			const user = userEvent.setup()
			const fetchMock = await open(
				{ agentId: 'a-dev', tab: 'skills' },
				{},
				{ ...DEV, skills: ['humanizer', 'fix'] },
			)
			expect(screen.getByText('2 activées')).toBeTruthy()
			expect(screen.queryByText(/sur 86/)).toBeNull()
			await user.click(screen.getByRole('button', { name: 'Retirer fix' }))
			expect(await screen.findByText('fix désactivée')).toBeTruthy()
			expect(patches(fetchMock)[0]?.body).toEqual({
				skills: ['humanizer'],
				change_summary: 'compétences',
			})
		})

		it('adds a free-text skill and a suggestion from the catalog', async () => {
			const user = userEvent.setup()
			const fetchMock = await open(
				{ agentId: 'a-dev', tab: 'skills' },
				{
					'GET /api/v1/skills': {
						items: [skill('ui-ux'), skill('humanizer')],
					},
				},
			)
			expect(
				await screen.findByRole('button', { name: 'Ajouter humanizer' }),
			).toBeTruthy()
			expect(
				screen.queryByRole('button', { name: 'Ajouter paperclip' }),
			).toBeNull()
			expect(screen.getByText('Aucune compétence')).toBeTruthy()
			await user.type(
				screen.getByLabelText('Nom de la compétence'),
				'mon-skill',
			)
			await user.click(screen.getByRole('button', { name: 'Ajouter' }))
			expect(await screen.findByText('mon-skill activée')).toBeTruthy()
			expect(patches(fetchMock)[0]?.body.skills).toEqual(['mon-skill'])
			await user.click(screen.getByRole('button', { name: 'Ajouter ui-ux' }))
			await waitFor(() => expect(patches(fetchMock)).toHaveLength(2))
			expect(patches(fetchMock)[1]?.body.skills).toEqual(['ui-ux'])
		})
	})

	it('sources: shows an empty state when no source targets the agent', async () => {
		await open(
			{ agentId: 'a-legal', tab: 'sources' },
			{
				'GET /api/v1/sources': {
					items: [source('s-other', { default_agent_id: 'a-dev' })],
				},
			},
			LEGAL,
		)
		expect(await screen.findByText('Aucune source branchée')).toBeTruthy()
	})

	it('sources: lists the sources whose default agent is this agent', async () => {
		await open(
			{ agentId: 'a-legal', tab: 'sources' },
			{
				'GET /api/v1/sources': {
					items: [
						source('s-dns', {
							name: 'Sous-domaines',
							default_agent_id: 'a-legal',
							events_last_24h: 7,
						}),
						source('s-other', { name: 'Autre', default_agent_id: 'a-dev' }),
					],
				},
			},
			LEGAL,
		)
		const link = await screen.findByRole('link', { name: 'Sous-domaines' })
		expect(link.getAttribute('href')).toBe('/sources/s-dns')
		expect(screen.getByText('7 / 24 h')).toBeTruthy()
		expect(screen.queryByRole('link', { name: 'Autre' })).toBeNull()
	})

	describe('delegation', () => {
		it('shows the parent and the sub-agents from reports_to', async () => {
			await open(
				{ agentId: 'a-legal', tab: 'team' },
				{
					'GET /api/v1/cards': {
						items: [
							card('c0ffee00-0000-4000-8000-000000000002', {
								assignee_agent_id: 'a-registre',
							}),
						],
					},
				},
				LEGAL,
			)
			expect(
				(await screen.findAllByRole('link', { name: /Registre/ })).length,
			).toBeGreaterThan(0)
			expect(
				screen.getAllByRole('link', { name: /Chief of Staff/ }).length,
			).toBeGreaterThan(0)
			expect(await screen.findByText('1 ouverte')).toBeTruthy()
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
				body: { reports_to: 'a-chief', change_summary: 'hiérarchie' },
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
				body: { reports_to: 'a-legal', change_summary: 'hiérarchie' },
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

		it('lists delegated sub-tasks from the cards', async () => {
			const parentCard = card('c0ffee00-0000-4000-8000-000000000003', {
				assignee_agent_id: 'a-legal',
			})
			await open(
				{ agentId: 'a-legal', tab: 'team' },
				{
					'GET /api/v1/cards': {
						items: [
							parentCard,
							card('c0ffee00-0000-4000-8000-000000000004', {
								title: 'Sous-tâche registre',
								parent_id: parentCard.id,
								assignee_agent_id: 'a-registre',
							}),
						],
					},
				},
				LEGAL,
			)
			expect(await screen.findByText('Sous-tâche registre')).toBeTruthy()
			expect(screen.getByText('Parent c0ffee00')).toBeTruthy()
		})

		it('rules form saves into the config', async () => {
			const user = userEvent.setup()
			const fetchMock = await open({ agentId: 'a-dev', tab: 'team' })
			await user.click(screen.getByRole('switch', { name: 'Peut déléguer' }))
			await user.selectOptions(
				screen.getByLabelText('Profondeur maximale'),
				'3',
			)
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			await waitFor(() => expect(patches(fetchMock)).toHaveLength(1))
			expect(patches(fetchMock)[0]?.body).toMatchObject({
				config: { canDeleg: true, dDepth: '3' },
				change_summary: 'droit de déléguer, profondeur de délégation',
			})
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
				change_summary: 'connecteurs',
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
			expect(patches(fetchMock)[0]?.body).toEqual({
				connector_grants: [],
				change_summary: 'connecteurs',
			})
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
				change_summary: 'connecteurs',
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
