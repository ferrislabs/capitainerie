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

		expect(input.getAttribute('accept')).toBe('.txt,.md,.vtt,.srt')
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
})
