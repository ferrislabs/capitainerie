import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { useState } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { Composer } from '#/domain/chat/ui/composer'

vi.mock('@tanstack/react-router', () => ({
	Link: ({ to, children }: { to: string; children: React.ReactNode }) => (
		<a href={to}>{children}</a>
	),
}))

const IDLE_VOICE = {
	state: 'idle',
	supported: true,
	onToggle: vi.fn(),
	onCancel: vi.fn(),
} as const

function Harness(
	props: Partial<Parameters<typeof Composer>[0]> & { initial?: string },
) {
	const [value, setValue] = useState(props.initial ?? '')
	return (
		<Composer
			label="Message"
			placeholder="Écrire…"
			value={value}
			onChange={setValue}
			onSend={vi.fn()}
			voice={IDLE_VOICE}
			model={{ kind: 'ready', model: 'claude-sonnet-5-5' }}
			{...props}
		/>
	)
}

describe('Composer keyboard', () => {
	it('sends on Enter and keeps the text for the parent to clear', async () => {
		const onSend = vi.fn()
		render(<Harness onSend={onSend} initial="Ajoute un README" />)

		await userEvent.type(screen.getByLabelText('Message'), '{Enter}')

		expect(onSend).toHaveBeenCalledTimes(1)
		expect(
			(screen.getByLabelText('Message') as HTMLTextAreaElement).value,
		).toBe('Ajoute un README')
	})

	it('inserts a new line on Shift+Enter without sending', async () => {
		const onSend = vi.fn()
		render(<Harness onSend={onSend} initial="Ligne 1" />)

		await userEvent.type(
			screen.getByLabelText('Message'),
			'{Shift>}{Enter}{/Shift}Ligne 2',
		)

		expect(onSend).not.toHaveBeenCalled()
		expect(
			(screen.getByLabelText('Message') as HTMLTextAreaElement).value,
		).toBe('Ligne 1\nLigne 2')
	})

	it('does not send an empty message, while streaming, or when disabled', async () => {
		const onSend = vi.fn()
		const { rerender } = render(<Harness onSend={onSend} />)
		await userEvent.type(screen.getByLabelText('Message'), '   {Enter}')
		expect(onSend).not.toHaveBeenCalled()

		rerender(<Harness onSend={onSend} initial="Salut" streaming />)
		await userEvent.type(screen.getByLabelText('Message'), '{Enter}')
		expect(onSend).not.toHaveBeenCalled()

		rerender(<Harness onSend={onSend} initial="Salut" disabled />)
		expect(
			(screen.getByRole('button', { name: 'Envoyer' }) as HTMLButtonElement)
				.disabled,
		).toBe(true)
	})

	it('aborts the turn on Escape only while streaming', async () => {
		const onAbort = vi.fn()
		const { rerender } = render(<Harness onAbort={onAbort} initial="x" />)
		await userEvent.type(screen.getByLabelText('Message'), '{Escape}')
		expect(onAbort).not.toHaveBeenCalled()

		rerender(<Harness onAbort={onAbort} initial="x" streaming />)
		await userEvent.type(screen.getByLabelText('Message'), '{Escape}')
		expect(onAbort).toHaveBeenCalledTimes(1)
	})

	it('cancels the dictation on Escape while recording', async () => {
		const onCancel = vi.fn()
		render(
			<Harness
				voice={{ ...IDLE_VOICE, state: 'recording', onCancel }}
				initial="x"
			/>,
		)

		await userEvent.type(screen.getByLabelText('Message'), '{Escape}')

		expect(onCancel).toHaveBeenCalledTimes(1)
	})

	it('offers Interrompre while streaming and calls onAbort', async () => {
		const onAbort = vi.fn()
		render(<Harness onAbort={onAbort} streaming />)

		await userEvent.click(screen.getByRole('button', { name: 'Interrompre' }))

		expect(onAbort).toHaveBeenCalledTimes(1)
		expect(screen.queryByRole('button', { name: 'Envoyer' })).toBeNull()
	})
})

describe('Composer controls', () => {
	it('shows the Captain model on the right and the mic button', async () => {
		const onToggle = vi.fn()
		render(<Harness voice={{ ...IDLE_VOICE, onToggle }} />)

		expect(screen.getByText('claude-sonnet-5-5')).toBeTruthy()
		await userEvent.click(screen.getByRole('button', { name: 'Dicter' }))
		expect(onToggle).toHaveBeenCalledTimes(1)
	})

	it('names the mic by what it will do and says what happens', () => {
		const { rerender } = render(
			<Harness voice={{ ...IDLE_VOICE, state: 'recording' }} />,
		)
		expect(
			screen
				.getByRole('button', { name: 'Arrêter la dictée' })
				.getAttribute('aria-pressed'),
		).toBe('true')
		expect(screen.getByText(/Enregistrement/)).toBeTruthy()

		rerender(<Harness voice={{ ...IDLE_VOICE, state: 'transcribing' }} />)
		expect(
			screen.getByRole('button', { name: 'Annuler la transcription' }),
		).toBeTruthy()
		expect(screen.getByText('Transcription…')).toBeTruthy()
	})

	it('hides the mic where recording is not supported', () => {
		render(<Harness voice={{ ...IDLE_VOICE, supported: false }} />)

		expect(screen.queryByRole('button', { name: 'Dicter' })).toBeNull()
	})

	it('links to the settings when no model is chosen', () => {
		render(<Harness model={{ kind: 'unknown' }} />)

		expect(
			screen
				.getByRole('link', { name: 'Modèle non choisi' })
				.getAttribute('href'),
		).toBe('/settings/models')
	})
})
