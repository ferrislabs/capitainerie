import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { SourceDetailPage } from '#/domain/sources/feature/source-detail-page'
import { source, sourceEvent } from '#/domain/sources/test-fixtures'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { DEV } from '#/test-utils/team-agents'

const ID = 's1'
const ROUTE = [
	'/sources/$sourceId/{-$tab}',
	'/agents/$agentId/{-$tab}',
	'/tasks/$taskId',
]
const CARD = '6f1c2a90-1111-4000-8000-000000000001'
const SOURCE = source(ID, {
	name: 'Pièces Chatwoot',
	slug: 'pieces-chatwoot',
	description: 'Pièces jointes du support',
	default_agent_id: DEV.id,
	auto_run: true,
	events_last_24h: 3,
})

type Routes = Parameters<typeof mockApi>[0]

async function open(tab?: string, routes: Routes = {}) {
	const fetchMock = mockApi({
		[`GET /api/v1/sources/${ID}`]: SOURCE,
		[`GET /api/v1/sources/${ID}/events`]: {
			items: [
				sourceEvent('e1', {
					status: 'card_created',
					summary: 'Facture reçue',
					card_id: CARD,
				}),
				sourceEvent('e2', {
					status: 'failed',
					summary: 'Relance',
					error: 'carte non créée : refus',
				}),
			],
			total: 2,
		},
		'GET /api/v1/team/agents': { items: [DEV] },
		'GET /api/v1/projects': { items: [] },
		...routes,
	})
	const path = tab ? `/sources/${ID}/${tab}` : `/sources/${ID}`
	await renderAtPath(
		withQueryClient(<SourceDetailPage sourceId={ID} tab={tab} />),
		path,
		ROUTE,
	)
	await screen.findByRole('heading', { name: 'Pièces Chatwoot' })
	return fetchMock
}

function calls(fetchMock: ReturnType<typeof mockApi>, method: string) {
	return fetchMock.mock.calls
		.filter(([, init]) => (init?.method ?? 'GET') === method)
		.map(([url, init]) => ({
			url: new URL(String(url)),
			body: init?.body ? JSON.parse(String(init.body)) : undefined,
		}))
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

describe('SourceDetailPage', () => {
	it('shows the configuration and a masked webhook URL', async () => {
		await open()
		const nav = screen.getByRole('navigation', {
			name: 'Sections de la source',
		})
		expect(
			within(nav)
				.getAllByRole('link')
				.map((l) => l.textContent),
		).toEqual(["Vue d'ensemble", 'Événements', 'Paramètres'])
		expect(screen.getByText('Active')).toBeTruthy()
		expect(screen.getByText('Pièces jointes du support')).toBeTruthy()
		expect(
			(await screen.findByRole('link', { name: DEV.name })).getAttribute(
				'href',
			),
		).toBe(`/agents/${DEV.id}`)
		expect(
			screen.getByText(
				'URL masquée, régénérez le token pour en obtenir une nouvelle.',
			),
		).toBeTruthy()
		expect(screen.queryByText(/démonstration/i)).toBeNull()
		expect(screen.queryByText(/Pipeline/)).toBeNull()
	})

	it('rotates the token after confirmation and shows the new URL once', async () => {
		const user = userEvent.setup()
		const fetchMock = await open(undefined, {
			[`POST /api/v1/sources/${ID}/rotate-token`]: {
				source: SOURCE,
				token: 'tok_new',
				webhook_url: 'https://api.test/hooks/sources/tok_new',
			},
		})
		await user.click(screen.getByRole('button', { name: 'Régénérer le token' }))
		const dialog = await screen.findByRole('alertdialog')
		expect(within(dialog).getByText(/cessera de fonctionner/)).toBeTruthy()
		expect(calls(fetchMock, 'POST')).toHaveLength(0)
		await user.click(within(dialog).getByRole('button', { name: 'Régénérer' }))
		expect((await screen.findByLabelText('URL du webhook')).textContent).toBe(
			'https://api.test/hooks/sources/tok_new',
		)
		expect(screen.getByText(/ne sera plus affichée/)).toBeTruthy()
		await user.click(screen.getByRole('button', { name: "J'ai copié l'URL" }))
		expect(screen.queryByLabelText('URL du webhook')).toBeNull()
	})

	it('lists the events with status, task link and error', async () => {
		await open('evenements')
		const table = await screen.findByRole('table')
		expect(within(table).getByText('Tâche créée')).toBeTruthy()
		expect(within(table).getByText('Échec')).toBeTruthy()
		expect(within(table).getByText('carte non créée : refus')).toBeTruthy()
		expect(
			within(table)
				.getByRole('link', { name: '6f1c2a90' })
				.getAttribute('href'),
		).toBe(`/tasks/${CARD}`)
		expect(
			screen.queryByRole('navigation', { name: 'Pagination des événements' }),
		).toBeNull()
	})

	it('pages through the events', async () => {
		const user = userEvent.setup()
		const fetchMock = await open('evenements', {
			[`GET /api/v1/sources/${ID}/events`]: {
				items: Array.from({ length: 20 }, (_, i) => sourceEvent(`e${i}`)),
				total: 45,
			},
		})
		expect(await screen.findByText('1–20 sur 45')).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Suivants' }))
		await waitFor(() =>
			expect(
				calls(fetchMock, 'GET').some(
					({ url }) =>
						url.pathname.endsWith('/events') &&
						url.searchParams.get('offset') === '20' &&
						url.searchParams.get('limit') === '20',
				),
			).toBe(true),
		)
	})

	it('sends a test event and reloads the events', async () => {
		const user = userEvent.setup()
		const fetchMock = await open('evenements', {
			[`POST /api/v1/sources/${ID}/test`]: {
				status: 202,
				body: sourceEvent('e9'),
			},
		})
		await screen.findByRole('table')
		const before = calls(fetchMock, 'GET').filter(({ url }) =>
			url.pathname.endsWith('/events'),
		).length
		await user.click(
			screen.getByRole('button', { name: 'Envoyer un événement de test' }),
		)
		expect(await screen.findByText('Événement de test envoyé')).toBeTruthy()
		await waitFor(() =>
			expect(
				calls(fetchMock, 'GET').filter(({ url }) =>
					url.pathname.endsWith('/events'),
				).length,
			).toBeGreaterThan(before),
		)
	})

	it('saves the settings and clears the agent with null', async () => {
		const user = userEvent.setup()
		const fetchMock = await open('parametres', {
			[`PATCH /api/v1/sources/${ID}`]: SOURCE,
		})
		await screen.findByRole('option', { name: DEV.name })
		expect((screen.getByLabelText('Slug') as HTMLInputElement).value).toBe(
			'pieces-chatwoot',
		)
		await user.selectOptions(screen.getByLabelText('Agent par défaut'), '')
		await user.click(screen.getByRole('button', { name: 'Enregistrer' }))
		expect(await screen.findByText('Source enregistrée')).toBeTruthy()
		expect(calls(fetchMock, 'PATCH')[0]?.body).toEqual({
			default_agent_id: null,
			auto_run: false,
		})
	})

	it('archives the source', async () => {
		const user = userEvent.setup()
		const fetchMock = await open('parametres', {
			[`POST /api/v1/sources/${ID}/archive`]: { ...SOURCE, archived: true },
		})
		await user.click(screen.getByRole('button', { name: 'Archiver la source' }))
		expect(await screen.findByText('Source archivée')).toBeTruthy()
		expect(calls(fetchMock, 'POST')[0]?.url.pathname).toBe(
			`/api/v1/sources/${ID}/archive`,
		)
	})

	it('says when the source does not exist', async () => {
		mockApi({
			[`GET /api/v1/sources/${ID}`]: {
				status: 404,
				body: { code: 'not_found', message: 'source', status: 404 },
			},
		})
		await renderAtPath(
			withQueryClient(<SourceDetailPage sourceId={ID} />),
			`/sources/${ID}`,
			ROUTE,
		)
		expect(await screen.findByText('Source introuvable')).toBeTruthy()
	})
})
