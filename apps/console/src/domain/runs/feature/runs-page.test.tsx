import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { RunsPage } from '#/domain/runs/feature/runs-page'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

vi.mock('@tanstack/react-router', () => ({
	Link: ({ children }: { children: ReactNode }) => <a href="/">{children}</a>,
}))

const VTT = `WEBVTT

1
00:00:01.000 --> 00:00:03.000
<v Alice>On livre vendredi.</v>

2
00:00:03.000 --> 00:00:05.000
<v Bob>Je prépare le devis.</v>
`

describe('RunsPage intake', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>
	const posted: unknown[] = []

	beforeEach(() => {
		posted.length = 0
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		fetchMock.mockImplementation((url: URL, init: RequestInit) => {
			if (init.method === 'POST') {
				posted.push(JSON.parse(String(init.body)))
				return Promise.resolve(json(202, { transcript_id: 't', run_id: 'r' }))
			}
			if (url.pathname === '/api/v1/integrations/github') {
				return Promise.resolve(json(200, { app: null, installation: null }))
			}
			return Promise.resolve(json(200, { items: [], total: 0 }))
		})
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it('imports a vtt, fills the form and sends the metadata', async () => {
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })

		await user.upload(
			screen.getByLabelText(/Glissez un fichier ici/),
			new File([VTT], 'Comité du lundi.vtt'),
		)

		expect((screen.getByLabelText('Titre') as HTMLInputElement).value).toBe(
			'Comité du lundi',
		)
		expect(
			(screen.getByLabelText('Transcript') as HTMLTextAreaElement).value,
		).toBe('Alice : On livre vendredi.\nBob : Je prépare le devis.')
		expect(screen.getByText(/2 participants détectés/)).toBeTruthy()
		expect(screen.getByRole('button', { name: 'Retirer Alice' })).toBeTruthy()

		await user.type(screen.getByLabelText('Date de la réunion'), '2026-10-19')
		await user.type(screen.getByLabelText('Participants'), 'Claire,')
		await user.type(screen.getByLabelText('Dépôt par défaut'), 'acme/web')
		await user.click(screen.getByRole('button', { name: 'Lancer le run' }))

		await waitFor(() => expect(posted).toHaveLength(1))
		expect(posted[0]).toEqual({
			title: 'Comité du lundi',
			body: 'Alice : On livre vendredi.\nBob : Je prépare le devis.',
			meeting_date: '2026-10-19',
			participants: ['Alice', 'Bob', 'Claire'],
			repository: 'acme/web',
		})
	})

	it('keeps an existing title and drops the import on Retirer', async () => {
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })

		await user.type(screen.getByLabelText('Titre'), 'Mon titre')
		await user.upload(
			screen.getByLabelText(/Glissez un fichier ici/),
			new File([VTT], 'autre.vtt'),
		)
		expect((screen.getByLabelText('Titre') as HTMLInputElement).value).toBe(
			'Mon titre',
		)

		await user.click(screen.getByRole('button', { name: 'Retirer' }))

		expect(
			(screen.getByLabelText('Transcript') as HTMLTextAreaElement).value,
		).toBe('')
		expect(screen.queryByRole('button', { name: 'Retirer Alice' })).toBeNull()
	})

	it('refuses an unsupported file', async () => {
		const user = userEvent.setup({ applyAccept: false })
		render(<RunsPage />, { wrapper: harness.wrapper })

		await user.upload(
			screen.getByLabelText(/Glissez un fichier ici/),
			new File(['x'], 'notes.pdf'),
		)

		expect((await screen.findByRole('alert')).textContent).toMatch(
			/Format non pris en charge/,
		)
	})
})
