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
import { resetAgentConfigs } from '#/domain/agent-team/agent-config-store'
import { AgentTeamPage } from '#/domain/agent-team/feature/agent-team-page'
import { OrgChartPage } from '#/domain/agent-team/feature/org-chart-page'
import { demoStore, resetDemoState, updateAgent } from '#/domain/demo'

const EXTRA = [
	'/agents',
	'/agents/organigramme',
	'/agents/nouveau',
	'/agents/$agentId/{-$tab}',
]

describe('team list and org chart', () => {
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

	it('lists the team with banner, tabs and a new-agent link', async () => {
		updateAgent('dev', { status: 'running' })
		await renderAtPath(<AgentTeamPage />, '/agents', EXTRA)
		expect(await screen.findByRole('heading', { name: 'Équipe' })).toBeTruthy()
		expect(screen.getByText(/Données de démonstration/)).toBeTruthy()
		const nav = screen.getByRole('navigation', { name: 'Sections des agents' })
		expect(
			within(nav)
				.getByRole('link', { name: 'Équipe' })
				.getAttribute('aria-current'),
		).toBe('page')
		expect(
			screen.getByRole('link', { name: /Nouvel agent/ }).getAttribute('href'),
		).toBe('/agents/nouveau')
		expect(screen.getAllByText('en cours')).toHaveLength(
			demoStore.state.agents.filter((a) => a.status === 'running').length,
		)
		expect(screen.getAllByText('inactif').length).toBeGreaterThan(5)
		const link = screen.getByRole('link', { name: 'Configurer Chief of Staff' })
		expect(link.getAttribute('href')).toBe('/agents/chief-of-staff')
	})

	it('shows agents created during the session', async () => {
		demoStore.setState((s) => ({
			...s,
			agents: [
				...s.agents,
				{
					id: 'zoe',
					name: 'Zoé',
					role: 'PM',
					description: 'Nouvelle',
					status: 'idle',
					hue: 10,
					model: 'Claude Sonnet 5.5',
					reportsTo: null,
				},
			],
		}))
		await renderAtPath(<AgentTeamPage />, '/agents', EXTRA)
		expect(await screen.findByText('Zoé')).toBeTruthy()
	})

	it('draws the org chart with sub-agent counts', async () => {
		await renderAtPath(<OrgChartPage />, '/agents/organigramme', EXTRA)
		expect(
			await screen.findByRole('heading', { name: 'Organigramme' }),
		).toBeTruthy()
		expect(screen.getByText(/Données de démonstration/)).toBeTruthy()
		expect(screen.getByText('2 sous-agents')).toBeTruthy()
		expect(screen.getAllByText(/peut déléguer/).length).toBeGreaterThan(0)
		const registre = screen.getByRole('link', { name: /Registre/ })
		expect(registre.getAttribute('href')).toBe('/agents/registre')
		await userEvent.click(registre)
	})
})
