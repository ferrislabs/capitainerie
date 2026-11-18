import { screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { InboxPage } from '#/domain/inbox/feature/inbox-page'
import type { InboxTab } from '#/domain/inbox/tabs'
import { clearAuth } from '#/store/auth.store'
import {
	type MockRoutes,
	mockApi,
	withQueryClient,
} from '#/test-utils/api-mock'

const NOW = Date.now()
const ago = (minutes: number) => new Date(NOW - minutes * 60_000).toISOString()

const item = (id: string, extra: Record<string, unknown> = {}) => ({
	id,
	kind: 'card',
	title: `Élément ${id}`,
	body: 'Dev a besoin d’une décision',
	created_at: ago(30),
	unread: true,
	...extra,
})

const INBOX = {
	items: [
		item('i1', { card_id: 'c1' }),
		item('i2', { kind: 'approval', unread: false, read_at: ago(5) }),
		item('i3', { kind: 'routine' }),
	],
	next_cursor: 'cursor-2',
	total: 67,
	unread: 2,
}

const BASE: MockRoutes = {
	'GET /api/v1/inbox': INBOX,
	'POST /api/v1/inbox/read': { updated: 2, unread: 0 },
}

afterEach(() => {
	vi.unstubAllGlobals()
	clearAuth()
})

async function open(tab: InboxTab, routes: MockRoutes = BASE) {
	const fetchMock = mockApi(routes)
	await renderAtPath(
		withQueryClient(<InboxPage tab={tab} />),
		tab === 'tous' ? '/inbox' : `/inbox/${tab}`,
		['/inbox/{-$tab}', '/tasks/$taskId'],
	)
	await screen.findByRole('heading', { name: 'Boîte de réception' })
	return fetchMock
}

const calls = (fetchMock: ReturnType<typeof mockApi>, path: string) =>
	fetchMock.mock.calls
		.map(([input, init]) => ({ url: new URL(String(input)), init }))
		.filter(({ url }) => url.pathname === path)

describe('InboxPage', () => {
	it('lists the items from the API with the counters', async () => {
		await open('tous')
		expect(await screen.findByText('67 éléments · 2 non lus')).toBeTruthy()
		const nav = screen.getByRole('navigation', {
			name: 'Filtres de la boîte de réception',
		})
		expect(within(nav).getAllByRole('link')).toHaveLength(2)
		expect(
			within(nav)
				.getByRole('link', { name: 'Tous' })
				.getAttribute('aria-current'),
		).toBe('page')
		expect(screen.getAllByRole('listitem')).toHaveLength(3)
		expect(screen.getAllByRole('img', { name: 'Non lu' })).toHaveLength(2)
		expect(screen.queryByText(/démonstration/i)).toBeNull()
		expect(
			screen.getByRole('link', { name: /Élément i1/ }).getAttribute('href'),
		).toBe('/tasks/c1')
		expect(
			screen.getByRole('link', { name: /Élément i2/ }).getAttribute('href'),
		).toBe('/approvals')
	})

	it('asks only for unread items on the non-lus tab', async () => {
		const fetchMock = await open('non-lus')
		await screen.findByText('Élément i1')
		const request = calls(fetchMock, '/api/v1/inbox')[0]
		expect(request?.url.searchParams.get('unread_only')).toBe('true')
	})

	it('loads the next page with the cursor', async () => {
		const fetchMock = await open('tous')
		await userEvent.click(
			await screen.findByRole('button', { name: 'Charger plus' }),
		)
		await waitFor(() =>
			expect(calls(fetchMock, '/api/v1/inbox')).toHaveLength(2),
		)
		expect(
			calls(fetchMock, '/api/v1/inbox')[1]?.url.searchParams.get('cursor'),
		).toBe('cursor-2')
	})

	it('marks everything as read', async () => {
		const fetchMock = await open('tous')
		await userEvent.click(
			await screen.findByRole('button', { name: 'Tout marquer comme lu' }),
		)
		expect(await screen.findByText('2 éléments marqués comme lus')).toBeTruthy()
		const post = calls(fetchMock, '/api/v1/inbox/read')[0]
		expect(JSON.parse(String(post?.init?.body))).toEqual({ all: true })
	})

	it('marks an unread item as read when it is opened', async () => {
		const fetchMock = await open('tous')
		await userEvent.click(
			await screen.findByRole('link', { name: /Élément i3/ }),
		)
		await waitFor(() =>
			expect(calls(fetchMock, '/api/v1/inbox/read')).toHaveLength(1),
		)
		const post = calls(fetchMock, '/api/v1/inbox/read')[0]
		expect(JSON.parse(String(post?.init?.body))).toEqual({ ids: ['i3'] })
	})

	it('shows an empty state and disables the bulk action', async () => {
		await open('non-lus', {
			'GET /api/v1/inbox': { items: [], total: 0, unread: 0 },
		})
		expect(await screen.findByText('Rien ici')).toBeTruthy()
		expect(
			(
				screen.getByRole('button', {
					name: 'Tout marquer comme lu',
				}) as HTMLButtonElement
			).disabled,
		).toBe(true)
	})

	it('shows an error when the inbox cannot load', async () => {
		await open('tous', {
			'GET /api/v1/inbox': {
				status: 500,
				body: { code: 'x', message: 'x', status: 500 },
			},
		})
		expect(
			await screen.findByText('Impossible de charger la boîte de réception.'),
		).toBeTruthy()
	})
})
