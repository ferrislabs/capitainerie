import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import {
	resetAgentConfigs,
	updateAgentConfig,
} from '#/domain/agent-team/agent-config-store'
import { AgentTeamPage } from '#/domain/agent-team/feature/agent-team-page'
import { OrgChartPage } from '#/domain/agent-team/feature/org-chart-page'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { TEAM } from '#/test-utils/team-agents'

const EXTRA = [
	'/agents',
	'/agents/organigramme',
	'/agents/nouveau',
	'/agents/$agentId/{-$tab}',
]

const LIST = { 'GET /api/v1/team/agents': { items: TEAM } }

describe('team list and org chart', () => {
	beforeEach(resetAgentConfigs)
	afterEach(() => {
		vi.unstubAllGlobals()
		clearAuth()
	})

	it('lists the team from the API with tabs and a new-agent link', async () => {
		mockApi(LIST)
		await renderAtPath(withQueryClient(<AgentTeamPage />), '/agents', EXTRA)
		expect(await screen.findByRole('heading', { name: 'Agents' })).toBeTruthy()
		const nav = screen.getByRole('navigation', { name: 'Sections des agents' })
		expect(
			within(nav)
				.getByRole('link', { name: 'Tous' })
				.getAttribute('aria-current'),
		).toBe('page')
		expect(within(nav).getByRole('link', { name: 'Archivés' })).toBeTruthy()
		expect(
			within(nav)
				.getByRole('link', { name: 'Organigramme' })
				.getAttribute('href'),
		).toBe('/agents/organigramme')
		expect(
			screen.getByRole('link', { name: /Nouvel agent/ }).getAttribute('href'),
		).toBe('/agents/nouveau')
		const link = await screen.findByRole('link', {
			name: 'Configurer Chief of Staff',
		})
		expect(link.getAttribute('href')).toBe('/agents/a-chief')
		expect(screen.getAllByText('en cours')).toHaveLength(1)
		expect(screen.getAllByText('inactif')).toHaveLength(3)
	})

	it('shows a loading state, then an error state', async () => {
		mockApi({
			'GET /api/v1/team/agents': {
				status: 500,
				body: { code: 'internal', message: 'boom', status: 500 },
			},
		})
		await renderAtPath(withQueryClient(<AgentTeamPage />), '/agents', EXTRA)
		expect(screen.getByText('Chargement…')).toBeTruthy()
		expect(
			await screen.findByText('Impossible de charger les agents.'),
		).toBeTruthy()
	})

	it('shows the empty state when the team is empty', async () => {
		mockApi({ 'GET /api/v1/team/agents': { items: [] } })
		await renderAtPath(withQueryClient(<AgentTeamPage />), '/agents', EXTRA)
		expect(await screen.findByText('Aucun agent')).toBeTruthy()
	})

	it('filters running agents and shows the empty state for errors', async () => {
		mockApi(LIST)
		const { unmount } = await renderAtPath(
			withQueryClient(<AgentTeamPage filter="actifs" />),
			'/agents',
			EXTRA,
		)
		expect(await screen.findAllByText('en cours')).toHaveLength(1)
		expect(screen.queryByText('inactif')).toBeNull()
		unmount()
		await renderAtPath(
			withQueryClient(<AgentTeamPage filter="erreur" />),
			'/agents',
			EXTRA,
		)
		expect(await screen.findByText('Aucun agent')).toBeTruthy()
		expect(screen.getByText(/Aucun agent erreur pour l'instant/)).toBeTruthy()
	})

	it('filters paused agents from the local pause state', async () => {
		mockApi(LIST)
		updateAgentConfig('a-dev', { paused: true })
		await renderAtPath(
			withQueryClient(<AgentTeamPage filter="pause" />),
			'/agents',
			EXTRA,
		)
		expect(
			await screen.findByRole('link', {
				name: 'Configurer Dev (cycle autonome)',
			}),
		).toBeTruthy()
		expect(screen.getAllByRole('link', { name: /Configurer/ })).toHaveLength(1)
	})

	it('asks the API for archived agents on the archives filter', async () => {
		const fetchMock = mockApi({ 'GET /api/v1/team/agents': { items: [] } })
		await renderAtPath(
			withQueryClient(<AgentTeamPage filter="archives" />),
			'/agents',
			EXTRA,
		)
		expect(
			await screen.findByText(/Aucun agent archivés pour l'instant/),
		).toBeTruthy()
		expect(
			fetchMock.mock.calls.some(([url]) =>
				String(url).includes('archived=true'),
			),
		).toBe(true)
	})

	it('draws the org chart from reports_to', async () => {
		mockApi(LIST)
		await renderAtPath(
			withQueryClient(<OrgChartPage />),
			'/agents/organigramme',
			EXTRA,
		)
		expect(await screen.findByRole('heading', { name: 'Agents' })).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'Organigramme' })
				.getAttribute('aria-current'),
		).toBe('page')
		expect(await screen.findByText('2 sous-agents')).toBeTruthy()
		expect(screen.getByText('1 sous-agent')).toBeTruthy()
		const registre = screen.getByRole('link', { name: /Registre/ })
		expect(registre.getAttribute('href')).toBe('/agents/a-registre')
		await userEvent.click(registre)
	})

	it('shows an empty org chart and its error state', async () => {
		mockApi({ 'GET /api/v1/team/agents': { items: [] } })
		const { unmount } = await renderAtPath(
			withQueryClient(<OrgChartPage />),
			'/agents/organigramme',
			EXTRA,
		)
		expect(await screen.findByText('Aucun agent')).toBeTruthy()
		unmount()
		mockApi({
			'GET /api/v1/team/agents': {
				status: 500,
				body: { code: 'internal', message: 'boom', status: 500 },
			},
		})
		await renderAtPath(
			withQueryClient(<OrgChartPage />),
			'/agents/organigramme',
			EXTRA,
		)
		expect(
			await screen.findByText("Impossible de charger l'organigramme."),
		).toBeTruthy()
	})
})
