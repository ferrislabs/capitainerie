import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { renderAtPath } from '#/components/layout/test-router'
import { AgentDetailPage } from '#/domain/agent-team/feature/agent-detail-page'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { chooseValue } from '#/test-utils/combobox'
import { DEV, teamAgent, teamRoutes } from '#/test-utils/team-agents'

const EXTRA = ['/agents/$agentId/{-$tab}']

const repository = (
	full_name: string,
	isPrivate = false,
): Schemas.RepositoryResponse => {
	const [owner = '', name = ''] = full_name.split('/')
	return {
		owner,
		name,
		full_name,
		private: isPrivate,
		html_url: `https://github.com/${full_name}`,
	}
}

const CONNECTED = {
	'GET /api/v1/integrations/github': {
		app: null,
		installation: {
			account_login: 'acme',
			installation_id: 7,
			connected_by: 'u',
			connected_at: '2026-11-01T00:00:00Z',
		},
	},
	'GET /api/v1/integrations/github/repositories': {
		repositories: [
			repository('acme/web', true),
			repository('acme/api'),
			repository('acme/docs'),
		],
	},
}

async function open(
	agent: Schemas.TeamAgentResponse,
	routes: Parameters<typeof mockApi>[0],
) {
	const fetchMock = mockApi({
		...teamRoutes(agent),
		[`PATCH /api/v1/team/agents/${agent.id}`]: { ...agent, revision: 2 },
		'GET /api/v1/cards': { items: [] },
		'GET /api/v1/runs/active': { items: [] },
		'GET /api/v1/mcp-connectors': { items: [] },
		...routes,
	})
	await renderAtPath(
		withQueryClient(<AgentDetailPage agentId={agent.id} tab="connectors" />),
		`/agents/${agent.id}`,
		EXTRA,
	)
	return fetchMock
}

function patchBodies(fetchMock: ReturnType<typeof mockApi>) {
	return fetchMock.mock.calls
		.filter(([, init]) => init?.method === 'PATCH')
		.map(([, init]) => JSON.parse(String(init?.body)))
}

describe('agent repository grants', () => {
	afterEach(() => {
		vi.unstubAllGlobals()
		clearAuth()
	})

	it('lists the granted repositories and adds one from the reachable list', async () => {
		const user = userEvent.setup()
		const agent = teamAgent('a-dev', 'Dev (cycle autonome)', {
			repository_grants: ['acme/web'],
		})
		const fetchMock = await open(agent, CONNECTED)

		const list = await screen.findByRole('list', { name: 'Dépôts autorisés' })
		expect(within(list).getByText('acme/web')).toBeTruthy()
		expect(await within(list).findByText('Privé')).toBeTruthy()

		const picker = screen.getByRole('combobox', { name: 'Ajouter un dépôt' })
		await user.click(picker)
		const options = within(await screen.findByRole('listbox'))
			.getAllByRole('option')
			.map((option) => option.getAttribute('data-value'))
		expect(options).toEqual(['acme/api', 'acme/docs'])
		await chooseValue(user, picker, 'acme/docs')

		expect(
			await screen.findByText('Dev peut travailler sur acme/docs'),
		).toBeTruthy()
		expect(patchBodies(fetchMock)).toEqual([
			{
				repository_grants: ['acme/web', 'acme/docs'],
				change_summary: 'dépôts GitHub',
			},
		])
	})

	it('removes a granted repository', async () => {
		const user = userEvent.setup()
		const agent = teamAgent('a-dev', 'Dev (cycle autonome)', {
			repository_grants: ['acme/web', 'acme/api'],
		})
		const fetchMock = await open(agent, CONNECTED)

		await user.click(
			await screen.findByRole('button', { name: 'Retirer acme/web' }),
		)

		await waitFor(() =>
			expect(patchBodies(fetchMock)).toEqual([
				{ repository_grants: ['acme/api'], change_summary: 'dépôts GitHub' },
			]),
		)
		expect(
			await screen.findByText('Accès à acme/web retiré à Dev'),
		).toBeTruthy()
	})

	it('flags a granted repository the installation no longer reaches', async () => {
		const agent = teamAgent('a-dev', 'Dev (cycle autonome)', {
			repository_grants: ['acme/legacy'],
		})
		await open(agent, CONNECTED)

		expect(
			await screen.findByText("L'application GitHub n'a plus accès à ce dépôt"),
		).toBeTruthy()
	})

	it('links to the GitHub settings when GitHub is not connected', async () => {
		await open(DEV, {
			'GET /api/v1/integrations/github': { app: null, installation: null },
		})

		const link = await screen.findByRole('link', { name: 'Connecter GitHub' })
		expect(link.getAttribute('href')).toBe('/settings/integrations/github')
		expect(screen.getByText('Aucun dépôt')).toBeTruthy()
		expect(
			screen.queryByRole('combobox', { name: 'Ajouter un dépôt' }),
		).toBeNull()
	})
})
