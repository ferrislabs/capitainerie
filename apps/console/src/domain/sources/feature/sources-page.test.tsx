import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { SourcesPage } from '#/domain/sources/feature/sources-page'
import { source } from '#/domain/sources/test-fixtures'
import { clearAuth } from '#/store/auth.store'
import { mockApi, withQueryClient } from '#/test-utils/api-mock'
import { DEV } from '#/test-utils/team-agents'

const EXTRA = ['/sources/nouvelle', '/sources/$sourceId/{-$tab}']

const CHATWOOT = source('s-chatwoot', {
	name: 'Pièces Chatwoot',
	slug: 'pieces-chatwoot',
	default_agent_id: DEV.id,
	events_last_24h: 41,
	last_event_at: new Date(Date.now() - 5 * 60_000).toISOString(),
})
const GITLAB = source('s-gitlab', {
	name: 'GitLab',
	description: 'merge requests',
	enabled: false,
})

type Routes = Parameters<typeof mockApi>[0]

async function open(routes: Routes = {}) {
	const fetchMock = mockApi({
		'GET /api/v1/sources': { items: [CHATWOOT, GITLAB] },
		'GET /api/v1/team/agents': { items: [DEV] },
		...routes,
	})
	await renderAtPath(withQueryClient(<SourcesPage />), '/sources', EXTRA)
	await screen.findByRole('link', { name: 'Pièces Chatwoot' })
	return fetchMock
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

describe('SourcesPage', () => {
	it('lists the real sources without demo banner', async () => {
		await open()
		expect(screen.getByRole('heading', { name: 'Sources' })).toBeTruthy()
		expect(screen.getByText(/2 sources · 1 active/)).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'Pièces Chatwoot' })
				.getAttribute('href'),
		).toBe('/sources/s-chatwoot')
		const row = screen
			.getByRole('link', { name: 'Pièces Chatwoot' })
			.closest('tr') as HTMLElement
		expect(within(row).getByText(DEV.name)).toBeTruthy()
		expect(within(row).getByText('41')).toBeTruthy()
		expect(within(row).getByText('il y a 5 min')).toBeTruthy()
		const other = screen.getByRole('link', { name: 'GitLab' }).closest('tr')
		expect(within(other as HTMLElement).getByText('Jamais')).toBeTruthy()
		expect(within(other as HTMLElement).getByText('Aucun agent')).toBeTruthy()
		expect(screen.queryByText(/démonstration/i)).toBeNull()
	})

	it('filters by search', async () => {
		const user = userEvent.setup()
		await open()
		await user.type(screen.getByLabelText('Rechercher une source'), 'merge')
		expect(screen.queryByRole('link', { name: 'Pièces Chatwoot' })).toBeNull()
		expect(screen.getByRole('link', { name: 'GitLab' })).toBeTruthy()
		await user.type(screen.getByLabelText('Rechercher une source'), 'zzz')
		expect(screen.getByText('Aucune source ne correspond')).toBeTruthy()
	})

	it('asks for the archived sources', async () => {
		const user = userEvent.setup()
		const fetchMock = await open()
		await user.click(screen.getByRole('button', { name: 'Voir les archivées' }))
		await waitFor(() =>
			expect(
				fetchMock.mock.calls.some(
					([url]) =>
						new URL(String(url)).searchParams.get('archived') === 'true',
				),
			).toBe(true),
		)
	})

	it('pauses a source through the API', async () => {
		const user = userEvent.setup()
		const fetchMock = await open({
			'PATCH /api/v1/sources/s-chatwoot': { ...CHATWOOT, enabled: false },
		})
		await user.click(
			screen.getByRole('switch', { name: 'Activer Pièces Chatwoot' }),
		)
		expect(
			await screen.findByText('Source en pause : Pièces Chatwoot'),
		).toBeTruthy()
		const patch = fetchMock.mock.calls.find(
			([, init]) => init?.method === 'PATCH',
		)
		expect(JSON.parse(String(patch?.[1]?.body))).toEqual({ enabled: false })
	})

	it('shows an empty state', async () => {
		mockApi({
			'GET /api/v1/sources': { items: [] },
			'GET /api/v1/team/agents': { items: [] },
		})
		await renderAtPath(withQueryClient(<SourcesPage />), '/sources', EXTRA)
		expect(await screen.findByText('Aucune source')).toBeTruthy()
	})
})
