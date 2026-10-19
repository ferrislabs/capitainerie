import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { RunsPage } from '#/domain/runs/feature/runs-page'
import { MAX_AUDIO_BYTES } from '#/domain/runs/transcript-import'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

vi.mock('@tanstack/react-router', () => ({
	Link: ({ to, children }: { to: string; children: ReactNode }) => (
		<a href={to}>{children}</a>
	),
}))

const ROLE = { provider: 'mistral', model: 'm' }
const ASSIGNMENTS = {
	extractor: ROLE,
	judge: ROLE,
	writer: ROLE,
	embeddings: ROLE,
}
const WITH_TRANSCRIBER = {
	assignments: {
		...ASSIGNMENTS,
		transcriber: { provider: 'mistral', model: 'voxtral-mini-latest' },
	},
}

describe('RunsPage audio intake', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>
	let assignments: unknown
	let transcribe: (url: URL, init: RequestInit) => Promise<Response>
	const transcriptions: Array<{ url: URL; init: RequestInit }> = []

	beforeEach(() => {
		transcriptions.length = 0
		assignments = WITH_TRANSCRIBER
		transcribe = () =>
			Promise.resolve(
				json(200, { text: 'Bonjour à tous', model: 'voxtral-mini-latest' }),
			)
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		fetchMock.mockImplementation((url: URL | string, init: RequestInit) => {
			const target = new URL(String(url))
			if (target.pathname === '/api/v1/transcriptions') {
				transcriptions.push({ url: target, init })
				return transcribe(target, init)
			}
			if (target.pathname === '/api/v1/model-assignments') {
				return Promise.resolve(json(200, assignments))
			}
			if (target.pathname === '/api/v1/integrations/github') {
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

	it('transcribes an audio file into the Transcript field', async () => {
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })

		await user.upload(
			screen.getByLabelText(/Glissez un fichier ici/),
			new File(['abc'], 'comité.mp3', { type: 'audio/mpeg' }),
		)

		await waitFor(() =>
			expect(
				(screen.getByLabelText('Transcript') as HTMLTextAreaElement).value,
			).toBe('Bonjour à tous'),
		)
		expect((screen.getByLabelText('Titre') as HTMLInputElement).value).toBe(
			'comité',
		)
		expect(screen.getByText(/transcrit par voxtral-mini-latest/)).toBeTruthy()
		expect(transcriptions).toHaveLength(1)
		const form = transcriptions[0].init.body as FormData
		expect((form.get('file') as File).name).toBe('comité.mp3')
		expect(screen.queryByText('Transcription en cours…')).toBeNull()
	})

	it('shows the progress and lets the user cancel the request', async () => {
		let signal: AbortSignal | undefined
		transcribe = (_url, init) =>
			new Promise((_resolve, reject) => {
				signal = init.signal as AbortSignal
				signal.addEventListener('abort', () =>
					reject(new DOMException('Aborted', 'AbortError')),
				)
			})
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })
		await user.type(screen.getByLabelText('Transcript'), 'Brouillon')

		await user.upload(
			screen.getByLabelText(/Glissez un fichier ici/),
			new File(['abc'], 'long.m4a'),
		)

		const status = await screen.findByRole('status')
		expect(status.textContent).toContain('Transcription en cours…')
		expect(status.textContent).toContain('long.m4a')
		expect(
			(
				screen.getByRole('button', {
					name: 'Lancer le run',
				}) as HTMLButtonElement
			).disabled,
		).toBe(true)

		await user.click(screen.getByRole('button', { name: 'Annuler' }))

		await waitFor(() => expect(screen.queryByRole('status')).toBeNull())
		expect(signal?.aborted).toBe(true)
		expect(screen.queryByRole('alert')).toBeNull()
		expect(
			(screen.getByLabelText('Transcript') as HTMLTextAreaElement).value,
		).toBe('Brouillon')
	})

	it('without a transcriber, says so, links to the Models page and sends nothing', async () => {
		assignments = { assignments: ASSIGNMENTS }
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })

		const link = await screen.findByRole('link', {
			name: 'Choisir un transcripteur',
		})
		expect(link.getAttribute('href')).toBe('/settings/models')
		await user.upload(
			screen.getByLabelText(/Glissez un fichier ici/),
			new File(['abc'], 'a.wav'),
		)

		expect((await screen.findByRole('alert')).textContent).toMatch(
			/Aucun transcripteur n’est configuré/,
		)
		expect(transcriptions).toHaveLength(0)
	})

	it('refuses an audio file over 25 Mo before sending it', async () => {
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })
		const big = new File(['x'], 'big.ogg')
		Object.defineProperty(big, 'size', { value: MAX_AUDIO_BYTES + 1 })

		await user.upload(screen.getByLabelText(/Glissez un fichier ici/), big)

		expect((await screen.findByRole('alert')).textContent).toBe(
			'Le fichier dépasse 25 Mo.',
		)
		expect(transcriptions).toHaveLength(0)
	})

	it('shows the provider failure and leaves the form untouched', async () => {
		transcribe = () =>
			Promise.resolve(
				json(502, {
					code: 'bad_gateway',
					status: 502,
					message: 'the provider answered 401 (check the API key)',
				}),
			)
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })

		await user.upload(
			screen.getByLabelText(/Glissez un fichier ici/),
			new File(['abc'], 'a.webm'),
		)

		expect((await screen.findByRole('alert')).textContent).toContain(
			'the provider answered 401',
		)
		expect(
			(screen.getByLabelText('Transcript') as HTMLTextAreaElement).value,
		).toBe('')
	})
})
