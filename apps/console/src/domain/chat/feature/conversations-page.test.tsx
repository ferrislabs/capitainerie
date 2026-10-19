import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { ConversationsPage } from '#/domain/chat/feature/conversations-page'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

vi.mock('@tanstack/react-router', () => ({
	Link: ({
		to,
		params,
		children,
	}: {
		to: string
		params?: { id: string }
		children: ReactNode
	}) => <a href={params ? to.replace('$id', params.id) : to}>{children}</a>,
}))

function conversation(
	id: string,
	title: string,
): Schemas.ConversationSummaryResponse {
	return {
		id,
		title,
		model: 'claude-sonnet-5-5',
		input_tokens: 1000,
		output_tokens: 500,
		cost_micros: 0,
		full: false,
		archived: false,
		created_at: '2026-10-19T10:00:00Z',
		updated_at: '2026-10-19T10:00:00Z',
	}
}

describe('ConversationsPage', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it('lists the conversations with a link to each', async () => {
		fetchMock.mockResolvedValue(
			json(200, {
				conversations: [
					conversation('c2', 'Relancer Acme'),
					conversation('c1', 'Ajoute un README'),
				],
			}),
		)

		render(<ConversationsPage />, { wrapper: harness.wrapper })

		const rows = await screen.findAllByRole('row')
		expect(rows).toHaveLength(3)
		expect(
			within(rows[1])
				.getByRole('link', { name: 'Relancer Acme' })
				.getAttribute('href'),
		).toBe('/chat/c2')
		expect(within(rows[1]).getByText('claude-sonnet-5-5')).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'Nouvelle conversation' })
				.getAttribute('href'),
		).toBe('/')
	})

	it('points to the home page when there is none', async () => {
		fetchMock.mockResolvedValue(json(200, { conversations: [] }))

		render(<ConversationsPage />, { wrapper: harness.wrapper })

		expect(await screen.findByText('Aucune conversation')).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'Lancer une mission' })
				.getAttribute('href'),
		).toBe('/')
	})

	it('says when the list cannot be loaded', async () => {
		fetchMock.mockResolvedValue(
			json(500, { code: 'internal', status: 500, message: 'boom' }),
		)

		render(<ConversationsPage />, { wrapper: harness.wrapper })

		expect(
			await screen.findByText('Impossible de charger les conversations.'),
		).toBeTruthy()
	})

	it('archives a conversation and reloads the list', async () => {
		let archived = false
		fetchMock.mockImplementation((url: URL, init: RequestInit) => {
			if (init.method === 'PATCH') {
				archived = true
				expect(url.pathname).toBe('/api/v1/conversations/c1')
				expect(JSON.parse(String(init.body))).toEqual({ archived: true })
				return Promise.resolve(
					json(200, conversation('c1', 'Ajoute un README')),
				)
			}
			return Promise.resolve(
				json(200, {
					conversations: archived
						? []
						: [conversation('c1', 'Ajoute un README')],
				}),
			)
		})
		render(<ConversationsPage />, { wrapper: harness.wrapper })

		await userEvent.click(
			await screen.findByRole('button', { name: 'Archiver Ajoute un README' }),
		)

		await waitFor(() =>
			expect(screen.getByText('Aucune conversation')).toBeTruthy(),
		)
	})
})
