import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { SubmitTranscriptForm } from '#/domain/runs/ui/submit-transcript-form'

function setup(overrides: Partial<Parameters<typeof SubmitTranscriptForm>[0]>) {
	const props = {
		title: '',
		body: '',
		titleMaxLength: 200,
		bodyMaxLength: 200000,
		isPending: false,
		onTitleChange: vi.fn(),
		onBodyChange: vi.fn(),
		onFileSelected: vi.fn(),
		onSubmit: vi.fn(),
		...overrides,
	}
	render(<SubmitTranscriptForm {...props} />)
	return props
}

describe('SubmitTranscriptForm', () => {
	it('shows validation and server messages with counters', () => {
		setup({
			title: 'abc',
			titleError: 'Le titre est obligatoire.',
			serverError: 'Titre trop long',
		})

		expect(screen.getByText('Le titre est obligatoire.')).toBeTruthy()
		expect(screen.getByText('Titre trop long')).toBeTruthy()
		expect(screen.getByText('3 / 200')).toBeTruthy()
	})

	it('submits and forwards the picked file', async () => {
		const props = setup({})
		const user = userEvent.setup()

		await user.click(screen.getByRole('button', { name: 'Lancer le run' }))
		expect(props.onSubmit).toHaveBeenCalledOnce()

		const file = new File(['hello'], 'a.txt', { type: 'text/plain' })
		await user.upload(screen.getByLabelText(/Importer un fichier/), file)
		expect(props.onFileSelected).toHaveBeenCalledWith(file)
	})

	it('disables the form while pending', () => {
		setup({ isPending: true })

		expect(
			(screen.getByRole('button', { name: 'Envoi…' }) as HTMLButtonElement)
				.disabled,
		).toBe(true)
		expect((screen.getByLabelText('Titre') as HTMLInputElement).disabled).toBe(
			true,
		)
	})
})
