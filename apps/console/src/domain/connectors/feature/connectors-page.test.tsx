import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { renderAtPath } from '#/components/layout/test-router'
import { ConnectorsPage } from '#/domain/connectors/feature/connectors-page'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'

const EXTRA = ['/connectors/{-$tab}', '/agents/$agentId/{-$tab}']

function connector(
	id: string,
	extra: Partial<Schemas.McpConnectorResponse> = {},
): Schemas.McpConnectorResponse {
	return {
		id,
		slug: id,
		name: `Connecteur ${id}`,
		url: `https://${id}.example.com/mcp`,
		resource: `https://${id}.example.com/mcp`,
		issuer: `https://${id}.example.com`,
		scopes: [],
		archived: false,
		connected: false,
		expires_at: null,
		created_by: 'u',
		created_at: '2026-11-01T00:00:00Z',
		updated_at: '2026-11-01T00:00:00Z',
		...extra,
	}
}

const MESTIER = connector('mestier', {
	connected: true,
	expires_at: '2099-01-01T00:00:00Z',
})
const SLACK = connector('slack')

async function open(routes: Parameters<typeof mockApi>[0], tab?: string) {
	const fetchMock = mockApi({
		'GET /api/v1/mcp-connectors': { items: [MESTIER, SLACK] },
		...routes,
	})
	await renderAtPath(
		withQueryClient(<ConnectorsPage tab={tab} />),
		'/connectors',
		EXTRA,
	)
	return fetchMock
}

function calls(fetchMock: ReturnType<typeof mockApi>, method: string) {
	return fetchMock.mock.calls
		.filter(([, init]) => init?.method === method)
		.map(([url, init]) => ({
			path: new URL(String(url)).pathname,
			body: init?.body ? JSON.parse(String(init.body)) : undefined,
		}))
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

describe('ConnectorsPage', () => {
	it('lists real connectors with host, state and expiry', async () => {
		await open({})
		expect(
			screen.getByRole('heading', { name: 'Connecteurs', level: 1 }),
		).toBeTruthy()
		const cards = await screen.findAllByRole('article')
		expect(cards).toHaveLength(2)
		const first = cards[0] as HTMLElement
		expect(within(first).getByText('mestier.example.com')).toBeTruthy()
		expect(within(first).getByText('Connecté')).toBeTruthy()
		expect(within(first).getByText(/Expire le/)).toBeTruthy()
		expect(
			within(cards[1] as HTMLElement).getByText('Non connecté'),
		).toBeTruthy()
	})

	it('shows the empty state', async () => {
		await open({ 'GET /api/v1/mcp-connectors': { items: [] } })
		expect(await screen.findByText('Aucun connecteur')).toBeTruthy()
	})

	it('shows a load error', async () => {
		await open({
			'GET /api/v1/mcp-connectors': {
				status: 500,
				body: { code: 'boom', message: 'boom', status: 500 },
			},
		})
		expect(
			await screen.findByText('Impossible de charger les connecteurs.'),
		).toBeTruthy()
	})

	it('adds a connector', async () => {
		const user = userEvent.setup()
		const created = connector('nouveau', { name: 'Mestier' })
		const fetchMock = await open({
			'POST /api/v1/mcp-connectors': { status: 201, body: created },
		})
		await screen.findAllByRole('article')
		await user.click(
			screen.getByRole('button', { name: 'Ajouter un connecteur' }),
		)
		const url = screen.getByLabelText('URL du serveur MCP')
		expect(url.getAttribute('placeholder')).toBe('https://mcp.example.com/mcp')
		await user.type(url, 'https://mcp.example.com/mcp')
		await user.type(screen.getByLabelText(/^Nom/), 'Mestier')
		await user.click(screen.getByRole('button', { name: 'Ajouter' }))
		expect(
			await screen.findByText('Connecteur « Mestier » ajouté'),
		).toBeTruthy()
		expect(calls(fetchMock, 'POST')[0]?.body).toEqual({
			url: 'https://mcp.example.com/mcp',
			name: 'Mestier',
		})
	})

	it('reports a duplicate slug on 409', async () => {
		const user = userEvent.setup()
		await open({
			'POST /api/v1/mcp-connectors': {
				status: 409,
				body: { code: 'conflict', message: 'dup', status: 409 },
			},
		})
		await screen.findAllByRole('article')
		await user.click(
			screen.getByRole('button', { name: 'Ajouter un connecteur' }),
		)
		await user.type(
			screen.getByLabelText('URL du serveur MCP'),
			'https://mcp.example.com/mcp',
		)
		await user.click(screen.getByRole('button', { name: 'Ajouter' }))
		expect(await screen.findByText(/utilise déjà cet identifiant/)).toBeTruthy()
		expect(screen.getByRole('dialog')).toBeTruthy()
	})

	it('rejects a non-http URL without calling the API', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({})
		await screen.findAllByRole('article')
		await user.click(
			screen.getByRole('button', { name: 'Ajouter un connecteur' }),
		)
		await user.type(screen.getByLabelText('URL du serveur MCP'), 'nope')
		await user.click(screen.getByRole('button', { name: 'Ajouter' }))
		expect(screen.getByText(/doit commencer par https/)).toBeTruthy()
		expect(calls(fetchMock, 'POST')).toEqual([])
	})

	it('redirects to the authorization url on connect', async () => {
		const user = userEvent.setup()
		const assign = vi.fn()
		vi.stubGlobal('location', { ...window.location, assign })
		const fetchMock = await open({
			'POST /api/v1/mcp-connectors/slack/authorize': {
				authorization_url: 'https://idp.example.com/authorize?x=1',
			},
		})
		const cards = await screen.findAllByRole('article')
		await user.click(
			within(cards[1] as HTMLElement).getByRole('button', {
				name: 'Se connecter',
			}),
		)
		await waitFor(() =>
			expect(assign).toHaveBeenCalledWith(
				'https://idp.example.com/authorize?x=1',
			),
		)
		expect(calls(fetchMock, 'POST')).toHaveLength(1)
	})

	it('tests a connector and lists tools with read-only badges', async () => {
		const user = userEvent.setup()
		await open({
			'POST /api/v1/mcp-connectors/mestier/test': {
				tools: [
					{ name: 'search', read_only: true },
					{ name: 'create_ticket', read_only: false },
				],
			},
		})
		const cards = await screen.findAllByRole('article')
		await user.click(
			within(cards[0] as HTMLElement).getByRole('button', { name: 'Tester' }),
		)
		const search = await screen.findByText('search')
		expect(
			within(search.closest('li') as HTMLElement).getByText('lecture'),
		).toBeTruthy()
		const create = screen.getByText('create_ticket')
		expect(
			within(create.closest('li') as HTMLElement).getByText('écriture'),
		).toBeTruthy()
	})

	it('explains a failed test', async () => {
		const user = userEvent.setup()
		await open({
			'POST /api/v1/mcp-connectors/mestier/test': {
				status: 409,
				body: { code: 'conflict', message: 'x', status: 409 },
			},
		})
		const cards = await screen.findAllByRole('article')
		await user.click(
			within(cards[0] as HTMLElement).getByRole('button', { name: 'Tester' }),
		)
		expect(await screen.findByText(/connecte-toi d'abord/)).toBeTruthy()
	})

	it('disconnects', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			'DELETE /api/v1/mcp-connectors/mestier/connection': {
				status: 204,
				body: undefined,
			},
		})
		const cards = await screen.findAllByRole('article')
		await user.click(
			within(cards[0] as HTMLElement).getByRole('button', {
				name: 'Se déconnecter',
			}),
		)
		expect(await screen.findByText('Connecteur déconnecté')).toBeTruthy()
		expect(calls(fetchMock, 'DELETE')).toHaveLength(1)
	})

	it('archives a connector', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			'PATCH /api/v1/mcp-connectors/slack': { ...SLACK, archived: true },
		})
		const cards = await screen.findAllByRole('article')
		await user.click(
			within(cards[1] as HTMLElement).getByRole('button', { name: 'Archiver' }),
		)
		expect(await screen.findByText('Connecteur archivé')).toBeTruthy()
		expect(calls(fetchMock, 'PATCH')[0]?.body).toEqual({ archived: true })
	})

	it('shows a success banner after the OAuth return', async () => {
		mockApi({ 'GET /api/v1/mcp-connectors': { items: [MESTIER] } })
		await renderAtPath(
			withQueryClient(<ConnectorsPage tab={undefined} connectedId="mestier" />),
			'/connectors',
			EXTRA,
		)
		expect(
			await screen.findByText('Connecteur « Connecteur mestier » connecté.'),
		).toBeTruthy()
	})
})
