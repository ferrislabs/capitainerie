import { screen } from '@testing-library/react'
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
import { AgentWizardPage } from '#/domain/agent-team/feature/agent-wizard-page'
import { demoStore, resetDemoState } from '#/domain/demo'

const EXTRA = ['/agents', '/agents/nouveau', '/agents/$agentId/{-$tab}']

describe('agent wizard', () => {
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

	it('gates steps, offers twelve adapters and creates the agent', async () => {
		const user = userEvent.setup()
		await renderAtPath(<AgentWizardPage />, '/agents/nouveau', EXTRA)
		expect(await screen.findByText(/Données de démonstration/)).toBeTruthy()

		const next = () => screen.getByRole('button', { name: /→$/ })
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
			screen
				.getByRole('checkbox', { name: 'Chatwoot' })
				.getAttribute('aria-checked'),
		).toBe('false')
		expect(screen.queryByRole('checkbox', { name: 'E-mail' })).toBeNull()
		await user.type(screen.getByLabelText('Titre'), 'Relais')
		await user.click(screen.getByRole('checkbox', { name: 'Chatwoot' }))
		await user.click(
			screen.getByRole('checkbox', { name: /Pièces reçues sur Chatwoot/ }),
		)
		await user.click(next())

		expect(screen.getByText('Darnold')).toBeTruthy()
		expect(screen.queryByText('Aucun (accès refusé)')).toBeNull()
		expect(screen.getByText('Chatwoot')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: "Tester l'agent" }))
		expect(screen.getByText(/Test réussi/)).toBeTruthy()
		await user.click(screen.getByRole('button', { name: "Créer l'agent" }))

		const created = demoStore.state.agents.find((a) => a.id === 'darnold')
		expect(created?.name).toBe('Darnold')
		expect(demoStore.state.grants.darnold).toEqual(['Chatwoot'])
		expect(
			demoStore.state.sources.find((s) => s.id === 'pieces-recues-sur-chatwoot')
				?.agentIds,
		).toContain('darnold')
		expect(getAgentConfig('darnold').fields.adapter).toBe('Codex')
	})

	it('goes back and keeps the answers', async () => {
		const user = userEvent.setup()
		await renderAtPath(<AgentWizardPage />, '/agents/nouveau', EXTRA)
		await user.type(await screen.findByLabelText("Nom de l'agent"), 'Zoé')
		await user.click(screen.getByRole('button', { name: /→$/ }))
		await user.click(screen.getByRole('button', { name: '← Retour' }))
		expect(
			(screen.getByLabelText("Nom de l'agent") as HTMLInputElement).value,
		).toBe('Zoé')
	})

	it('creates with no connector by default', async () => {
		const user = userEvent.setup()
		await renderAtPath(<AgentWizardPage />, '/agents/nouveau', EXTRA)
		await user.type(await screen.findByLabelText("Nom de l'agent"), 'Nu')
		await user.click(screen.getByRole('button', { name: /→$/ }))
		await user.click(screen.getByRole('button', { name: 'Pi' }))
		await user.click(screen.getByRole('button', { name: /→$/ }))
		await user.click(screen.getByRole('button', { name: /→$/ }))
		await user.click(screen.getByRole('button', { name: /→$/ }))
		expect(screen.getByText('Aucun (accès refusé)')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: "Créer l'agent" }))
		expect(demoStore.state.grants.nu ?? []).toEqual([])
	})
})
