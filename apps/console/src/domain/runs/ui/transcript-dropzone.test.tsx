import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { TranscriptDropzone } from '#/domain/runs/ui/transcript-dropzone'

function setup(
	overrides: Partial<Parameters<typeof TranscriptDropzone>[0]> = {},
) {
	const props = {
		isDragging: false,
		disabled: false,
		dropHandlers: {
			onDragEnter: vi.fn(),
			onDragOver: vi.fn(),
			onDragLeave: vi.fn(),
			onDrop: vi.fn(),
		},
		onFileSelected: vi.fn(),
		onRemove: vi.fn(),
		...overrides,
	}
	render(<TranscriptDropzone {...props} />)
	return props
}

describe('TranscriptDropzone', () => {
	it('lists the accepted formats and forwards the picked file', async () => {
		const props = setup()
		const user = userEvent.setup()
		const input = screen.getByLabelText(/Glissez un fichier ici/)

		expect(input.getAttribute('accept')).toBe(
			'.txt,.md,.vtt,.srt,.mp3,.m4a,.wav,.webm,.ogg,.aac',
		)
		const file = new File(['x'], 'a.vtt')
		await user.upload(input, file)

		expect(props.onFileSelected).toHaveBeenCalledWith(file)
	})

	it('wires the drag events on the drop area', () => {
		const props = setup()
		const area = screen.getByText(/Glissez un fichier ici/).closest('label')
		if (!area) throw new Error('drop area not found')

		fireEvent.dragEnter(area)
		fireEvent.drop(area)

		expect(props.dropHandlers.onDragEnter).toHaveBeenCalled()
		expect(props.dropHandlers.onDrop).toHaveBeenCalled()
	})

	it('previews the import and lets the user remove it', async () => {
		const props = setup({
			imported: { name: 'comite.vtt', characters: 12345, participants: 3 },
		})
		const user = userEvent.setup()

		expect(screen.getByText('comite.vtt')).toBeTruthy()
		expect(screen.getByText(/12\s345 caractères/)).toBeTruthy()
		expect(screen.getByText(/3 participants détectés/)).toBeTruthy()
		await user.click(screen.getByRole('button', { name: 'Retirer' }))

		expect(props.onRemove).toHaveBeenCalledOnce()
	})

	it('uses the singular for one participant', () => {
		setup({ imported: { name: 'a.srt', characters: 1, participants: 1 } })

		expect(screen.getByText(/1 caractère ·/)).toBeTruthy()
		expect(screen.getByText(/1 participant détecté$/)).toBeTruthy()
	})

	it('shows a file error', () => {
		setup({ error: 'Le fichier dépasse 5 Mo.' })

		expect(screen.getByRole('alert').textContent).toBe(
			'Le fichier dépasse 5 Mo.',
		)
	})

	it('announces the audio formats and their 25 Mo limit', () => {
		setup()

		expect(
			screen.getByText(/\.mp3, \.m4a, \.wav, \.webm, \.ogg ou \.aac, 25 Mo au plus/),
		).toBeTruthy()
		expect(screen.queryByText(/Aucun transcripteur/)).toBeNull()
	})

	it('says a transcriber is missing and links to the Models page', () => {
		setup({
			transcriberMissing: true,
			modelsLink: <a href="/settings/models">Choisir un transcripteur</a>,
		})

		expect(screen.getByText(/Aucun transcripteur n’est configuré/)).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'Choisir un transcripteur' })
				.getAttribute('href'),
		).toBe('/settings/models')
	})

	it('shows the transcription in progress with the file name and a cancel button', async () => {
		const onCancel = vi.fn()
		setup({ transcribingFile: 'réunion.m4a', onCancel })
		const user = userEvent.setup()

		const status = screen.getByRole('status')
		expect(status.textContent).toContain('Transcription en cours…')
		expect(status.textContent).toContain('réunion.m4a')
		expect(
			(screen.getByLabelText(/Glissez un fichier ici/) as HTMLInputElement)
				.disabled,
		).toBe(true)
		await user.click(screen.getByRole('button', { name: 'Annuler' }))

		expect(onCancel).toHaveBeenCalledOnce()
	})

	it('names the transcriber in the preview of a transcribed audio', () => {
		setup({
			imported: {
				name: 'a.mp3',
				characters: 20,
				participants: 0,
				transcribedBy: 'voxtral-mini-latest',
			},
		})

		expect(screen.getByText(/transcrit par voxtral-mini-latest/)).toBeTruthy()
		expect(screen.queryByText(/participant/)).toBeNull()
	})
})
