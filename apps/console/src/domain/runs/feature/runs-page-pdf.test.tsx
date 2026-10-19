import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { RunsPage } from '#/domain/runs/feature/runs-page'
import { MAX_PDF_BYTES } from '#/domain/runs/transcript-import'
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
const WITH_READER = {
	assignments: {
		...ASSIGNMENTS,
		reader: { provider: 'mistral', model: 'mistral-ocr-latest' },
	},
}

describe('RunsPage PDF intake', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>
	let assignments: unknown
	let readPdf: (url: URL, init: RequestInit) => Promise<Response>
	const reads: Array<{ url: URL; init: RequestInit }> = []

	beforeEach(() => {
		reads.length = 0
		assignments = WITH_READER
		readPdf = () =>
			Promise.resolve(
				json(200, {
					text: '# Rapport\n\nBonjour',
					model: 'mistral-ocr-latest',
					pages: 2,
				}),
			)
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
		fetchMock.mockImplementation((url: URL | string, init: RequestInit) => {
			const target = new URL(String(url))
			if (target.pathname === '/api/v1/documents/text') {
				reads.push({ url: target, init })
				return readPdf(target, init)
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

	it('reads a PDF into the Transcript field and titles it from the file name', async () => {
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })

		await user.upload(
			screen.getByLabelText(/Glissez un fichier ici/),
			new File(['%PDF-1'], 'compte-rendu.pdf', { type: 'application/pdf' }),
		)

		await waitFor(() =>
			expect(
				(screen.getByLabelText('Transcript') as HTMLTextAreaElement).value,
			).toBe('# Rapport\n\nBonjour'),
		)
		expect((screen.getByLabelText('Titre') as HTMLInputElement).value).toBe(
			'compte-rendu',
		)
		expect(screen.getByText(/lu par mistral-ocr-latest/)).toBeTruthy()
		expect(reads).toHaveLength(1)
		const form = reads[0].init.body as FormData
		expect((form.get('file') as File).name).toBe('compte-rendu.pdf')
		expect(screen.queryByText('Lecture du PDF en cours…')).toBeNull()
	})

	it('keeps a title that is already typed', async () => {
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })
		await user.type(screen.getByLabelText('Titre'), 'Comité')

		await user.upload(
			screen.getByLabelText(/Glissez un fichier ici/),
			new File(['%PDF-1'], 'autre.pdf'),
		)

		await waitFor(() =>
			expect(
				(screen.getByLabelText('Transcript') as HTMLTextAreaElement).value,
			).toContain('Rapport'),
		)
		expect((screen.getByLabelText('Titre') as HTMLInputElement).value).toBe(
			'Comité',
		)
	})

	it('shows the progress and lets the user cancel the request', async () => {
		let signal: AbortSignal | undefined
		readPdf = (_url, init) =>
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
			new File(['%PDF-1'], 'long.pdf'),
		)

		const status = await screen.findByRole('status')
		expect(status.textContent).toContain('Lecture du PDF en cours…')
		expect(status.textContent).toContain('long.pdf')
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

	it('without a reader, says so, links to the Models page and sends nothing', async () => {
		assignments = { assignments: ASSIGNMENTS }
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })

		const link = await screen.findByRole('link', {
			name: 'Choisir un lecteur de PDF',
		})
		expect(link.getAttribute('href')).toBe('/settings/models')
		await user.upload(
			screen.getByLabelText(/Glissez un fichier ici/),
			new File(['%PDF-1'], 'a.pdf'),
		)

		expect((await screen.findByRole('alert')).textContent).toMatch(
			/Aucun lecteur de PDF n’est configuré/,
		)
		expect(reads).toHaveLength(0)
	})

	it('refuses a PDF over 25 Mo before sending it', async () => {
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })
		const big = new File(['x'], 'big.pdf')
		Object.defineProperty(big, 'size', { value: MAX_PDF_BYTES + 1 })

		await user.upload(screen.getByLabelText(/Glissez un fichier ici/), big)

		expect((await screen.findByRole('alert')).textContent).toBe(
			'Le fichier dépasse 25 Mo.',
		)
		expect(reads).toHaveLength(0)
	})

	it('shows a fake PDF refusal from the API and leaves the form untouched', async () => {
		readPdf = () =>
			Promise.resolve(
				json(415, {
					code: 'unsupported_media_type',
					status: 415,
					message: 'the file must be a PDF',
				}),
			)
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })

		await user.upload(
			screen.getByLabelText(/Glissez un fichier ici/),
			new File(['pas un pdf'], 'faux.pdf'),
		)

		expect((await screen.findByRole('alert')).textContent).toBe(
			'Ce fichier n’est pas un PDF valide.',
		)
		expect(
			(screen.getByLabelText('Transcript') as HTMLTextAreaElement).value,
		).toBe('')
	})

	it('says when the PDF holds no text', async () => {
		readPdf = () =>
			Promise.resolve(
				json(200, { text: '  \n', model: 'mistral-ocr-latest', pages: 1 }),
			)
		const user = userEvent.setup()
		render(<RunsPage />, { wrapper: harness.wrapper })

		await user.upload(
			screen.getByLabelText(/Glissez un fichier ici/),
			new File(['%PDF-1'], 'scan.pdf'),
		)

		expect((await screen.findByRole('alert')).textContent).toBe(
			'Ce PDF ne contient aucun texte exploitable.',
		)
	})
})
