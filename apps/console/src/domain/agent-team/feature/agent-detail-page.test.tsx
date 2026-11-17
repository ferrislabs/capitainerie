import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import {
	afterEach,
	beforeEach,
	describe,
	expect,
	it,
	type MockInstance,
	vi,
} from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import {
	getAgentConfig,
	resetAgentConfigs,
} from '#/domain/agent-team/agent-config-store'
import { AGENT_TABS } from '#/domain/agent-team/agent-tabs'
import { AgentDetailPage } from '#/domain/agent-team/feature/agent-detail-page'
import { demoStore, resetDemoState } from '#/domain/demo'

const EXTRA = [
	'/sources/$sourceId/{-$tab}',
	'/agents/equipe',
	'/agents/organigramme',
	'/agents/equipe/$agentId/{-$tab}',
]

type Page = Parameters<typeof AgentDetailPage>[0]

async function open(props: Page) {
	await renderAtPath(
		<AgentDetailPage {...props} />,
		`/agents/equipe/${props.agentId}`,
		EXTRA,
	)
	await screen.findByRole('navigation', { name: "Sections de l'agent" })
}

describe('agent detail page', () => {
	let fetchSpy: MockInstance<typeof fetch>
	beforeEach(() => {
		resetDemoState()
		resetAgentConfigs()
		fetchSpy = vi.spyOn(globalThis, 'fetch')
	})
	afterEach(() => {
		expect(fetchSpy).not.toHaveBeenCalled()
		fetchSpy.mockRestore()
	})

	it('renders header, banner and eleven tab links with the active one marked', async () => {
		await open({ agentId: 'dev', tab: 'skills' })
		expect(
			screen.getByRole('heading', { name: 'Dev (cycle autonome)' }),
		).toBeTruthy()
		expect(screen.getByText(/Données de démonstration/)).toBeTruthy()
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
		).toBe('/runs')
		expect(
			within(nav).getByRole('link', { name: 'Coûts' }).getAttribute('href'),
		).toBe('/agents')
		for (const label of [
			'Définir comme principal',
			'Assigner une tâche',
			'Lancer',
			'Pause',
		])
			expect(screen.getByRole('button', { name: label })).toBeTruthy()
	})

	it('falls back to overview and maps legacy tabs', async () => {
		await open({ agentId: 'dev', tab: 'bogus' })
		expect(screen.getByText('Harness / Runtime')).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: "Vue d'ensemble" })
				.getAttribute('aria-current'),
		).toBe('page')
	})

	it('opens connectors for the legacy tools segment', async () => {
		await open({ agentId: 'dev', tab: 'tools' })
		expect(screen.getByText("Connecteurs de l'agent")).toBeTruthy()
	})

	it('toggles pause and shows a message when launching', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'dev' })
		await user.click(screen.getByRole('button', { name: 'Pause' }))
		expect(screen.getByRole('button', { name: 'Relancer' })).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Lancer' }))
		expect(screen.getByText('Exécution lancée pour Dev')).toBeTruthy()
	})

	it('shows a not-found state', async () => {
		await renderAtPath(
			<AgentDetailPage agentId="nope" />,
			'/agents/equipe/nope',
			EXTRA,
		)
		expect(await screen.findByText('Agent introuvable')).toBeTruthy()
	})

	describe('instructions', () => {
		it('reads, shows raw and edits', async () => {
			await open({ agentId: 'dev', tab: 'instructions', mode: 'lire' })
			expect(
				screen.getByRole('heading', { name: 'Dev · Natalia' }),
			).toBeTruthy()
			expect(
				screen.getByRole('link', { name: 'Éditer' }).getAttribute('href'),
			).toContain('mode=editer')
		})

		it('shows raw text', async () => {
			await open({ agentId: 'dev', tab: 'instructions', mode: 'brut' })
			expect(screen.getByText(/^# Dev · Natalia/)).toBeTruthy()
		})

		it('gives the legal agent its own instructions', async () => {
			await open({
				agentId: 'veille-legale',
				tab: 'instructions',
				mode: 'brut',
			})
			expect(screen.getByText(/Nouveaux sous-domaines sensibles/)).toBeTruthy()
		})

		it('saves an edit as a new revision listed in the revisions tab', async () => {
			const user = userEvent.setup()
			await open({ agentId: 'dev', tab: 'instructions', mode: 'editer' })
			expect(screen.queryByText('Modifications non enregistrées')).toBeNull()
			await user.type(screen.getByLabelText('Instructions'), ' plus')
			expect(screen.getByText('Modifications non enregistrées')).toBeTruthy()
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(screen.queryByText('Modifications non enregistrées')).toBeNull()
			expect(getAgentConfig('dev').rev).toBe(15)
			expect(getAgentConfig('dev').revs[0]?.changed).toBe('instructions')
			expect(getAgentConfig('dev').fields.instr.endsWith(' plus')).toBe(true)
		})

		it('cancels an edit', async () => {
			const user = userEvent.setup()
			await open({ agentId: 'dev', tab: 'instructions', mode: 'editer' })
			await user.type(screen.getByLabelText('Instructions'), 'x')
			await user.click(screen.getByRole('button', { name: 'Annuler' }))
			expect(screen.queryByText('Modifications non enregistrées')).toBeNull()
			expect(getAgentConfig('dev').rev).toBe(14)
		})
	})

	it('lists revisions with current and restore', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'dev', tab: 'rev' })
		expect(screen.getByText('Actuelle')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Restaurer Rév. 13' }))
		expect(screen.getByText(/Rév. 13 restaurée/)).toBeTruthy()
	})

	it('runtime changes bump the revision and name the fields', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'dev', tab: 'runtime' })
		await user.selectOptions(
			screen.getByLabelText('Effort de réflexion'),
			'Élevé',
		)
		await user.click(screen.getByRole('switch', { name: 'Activer Chrome' }))
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(getAgentConfig('dev').revs[0]?.changed).toBe(
			'effort de réflexion, Chrome',
		)
		expect(screen.getByText('Enregistré · révision 15 créée')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Lancer le test' }))
		expect(screen.getByText(/Test réussi/)).toBeTruthy()
	})

	it('edits secrets and permissions', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'dev', tab: 'secrets' })
		await user.click(
			screen.getByRole('button', { name: 'Ajouter une variable' }),
		)
		expect(screen.getAllByLabelText('Nom de la variable')).toHaveLength(2)
		await user.click(
			screen.getAllByRole('button', {
				name: 'Supprimer la variable',
			})[0] as HTMLElement,
		)
		expect(screen.getAllByLabelText('Nom de la variable')).toHaveLength(1)
		expect(screen.getByText('Modifications non enregistrées')).toBeTruthy()
	})

	it('permissions tab saves', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'dev', tab: 'perms' })
		await user.click(screen.getByRole('switch', { name: 'Créer des agents' }))
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(getAgentConfig('dev').fields.pCreate).toBe(true)
		expect(getAgentConfig('dev').revs[0]?.changed).toBe(
			'droit de créer des agents',
		)
	})

	it('skills: switches, count and autosave message', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'dev', tab: 'skills' })
		expect(screen.getByText('8 sur 11')).toBeTruthy()
		await user.click(screen.getByRole('switch', { name: 'Activer ui-ux' }))
		expect(screen.getByText('9 sur 11')).toBeTruthy()
		expect(screen.getByText('ui-ux activée')).toBeTruthy()
	})

	it('sources: detach and attach through the store', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'veille-legale', tab: 'sources' })
		expect(screen.getByText('Nouveaux sous-domaines sensibles')).toBeTruthy()
		expect(
			screen.getByText(/À chaque événement : une tâche est créée/),
		).toBeTruthy()
		expect(screen.getByText(/Sous-domaine \{hôte\}/)).toBeTruthy()
		await user.click(screen.getByRole('button', { name: /Arrêter d'écouter/ }))
		expect(
			demoStore.state.sources.find(
				(s) => s.id === 'nouveaux-sous-domaines-sensibles',
			)?.agentIds,
		).not.toContain('veille-legale')
		await user.selectOptions(
			screen.getByLabelText('Source'),
			'nouveaux-sous-domaines-sensibles',
		)
		await user.click(screen.getByRole('button', { name: 'Écouter' }))
		expect(
			demoStore.state.sources.find(
				(s) => s.id === 'nouveaux-sous-domaines-sensibles',
			)?.agentIds,
		).toContain('veille-legale')
	})

	describe('delegation', () => {
		it('shows ancestors, sub-agents with open tasks and the inheritance rule', async () => {
			await open({ agentId: 'veille-legale', tab: 'team' })
			expect(
				screen.getAllByRole('link', { name: /Registre/ }).length,
			).toBeGreaterThan(0)
			expect(screen.getAllByText(/ouverte/).length).toBeGreaterThan(0)
			expect(screen.getByText(/n'hérite d'aucun connecteur/)).toBeTruthy()
			expect(
				screen.getAllByRole('link', { name: /Chief of Staff/ }).length,
			).toBeGreaterThan(0)
			expect(screen.getAllByText(/Parent NAT-691/).length).toBeGreaterThan(0)
		})

		it('removes a sub-agent back to the Chief of Staff', async () => {
			const user = userEvent.setup()
			await open({ agentId: 'veille-legale', tab: 'team' })
			await user.click(screen.getByRole('button', { name: /Retirer Registre/ }))
			expect(demoStore.state.reportsTo.registre).toBe('Chief of Staff')
		})

		it('adds a sub-agent but never offers self or ancestors', async () => {
			const user = userEvent.setup()
			await open({ agentId: 'veille-legale', tab: 'team' })
			const select = screen.getByLabelText('Agent à ajouter')
			const names = within(select)
				.getAllByRole('option')
				.map((o) => o.textContent)
			expect(names).not.toContain('Veille légale')
			expect(names).not.toContain('Chief of Staff')
			await user.selectOptions(select, 'dev')
			await user.click(screen.getByRole('button', { name: 'Ajouter' }))
			expect(demoStore.state.reportsTo.dev).toBe('Veille légale')
		})

		it('rules form saves with a revision', async () => {
			const user = userEvent.setup()
			await open({ agentId: 'dev', tab: 'team' })
			await user.click(screen.getByRole('switch', { name: 'Peut déléguer' }))
			await user.selectOptions(
				screen.getByLabelText('Profondeur maximale'),
				'3',
			)
			await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
			expect(getAgentConfig('dev').revs[0]?.changed).toBe(
				'droit de déléguer, profondeur de délégation',
			)
		})
	})

	describe('connectors', () => {
		it('is deny by default and binds switches to the store', async () => {
			const user = userEvent.setup()
			await open({ agentId: 'anouk', tab: 'connectors' })
			const chatwoot = screen.getByRole('switch', {
				name: 'Autoriser Chatwoot',
			})
			expect(chatwoot.getAttribute('aria-checked')).toBe('false')
			await user.click(chatwoot)
			expect(demoStore.state.grants.anouk).toEqual(['Chatwoot'])
			expect(screen.getByText('Anouk peut utiliser Chatwoot')).toBeTruthy()
			expect(
				screen.getByText('chatwoot_reply', { selector: 'td' }),
			).toBeTruthy()
			await user.click(
				screen.getByRole('switch', { name: 'Autoriser Chatwoot' }),
			)
			expect(demoStore.state.grants.anouk).toEqual([])
		})

		it('refuses a connector without account', async () => {
			const user = userEvent.setup()
			await open({ agentId: 'anouk', tab: 'connectors' })
			await user.click(screen.getByRole('switch', { name: 'Autoriser Slack' }))
			expect(demoStore.state.grants.anouk ?? []).toEqual([])
			expect(
				screen.getByText(/Connecte d'abord un compte à « Slack »/),
			).toBeTruthy()
			expect(screen.getAllByText('Non connecté').length).toBeGreaterThan(0)
			expect(screen.getByText('Outils indisponibles')).toBeTruthy()
			expect(screen.getAllByText('Refusé').length).toBe(2)
			expect(screen.getByLabelText('Compte GitHub utilisé')).toBeTruthy()
		})
	})

	it('keys: create and revoke', async () => {
		const user = userEvent.setup()
		await open({ agentId: 'dev', tab: 'keys' })
		await user.click(screen.getByRole('button', { name: 'Créer une clé' }))
		expect(screen.getAllByRole('button', { name: /Révoquer/ })).toHaveLength(2)
		await user.click(
			screen.getByRole('button', { name: 'Révoquer Heartbeat CI' }),
		)
		expect(screen.getAllByRole('button', { name: /Révoquer/ })).toHaveLength(1)
	})
})
