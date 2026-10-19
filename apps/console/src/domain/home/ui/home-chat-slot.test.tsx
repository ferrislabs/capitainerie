import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { HomeChatSlot } from '#/domain/home/ui/home-chat-slot'

vi.mock('@tanstack/react-router', () => ({
	Link: ({ to, children }: { to: string; children: ReactNode }) => (
		<a href={to}>{children}</a>
	),
}))

const VOICE = {
	state: 'idle',
	supported: true,
	onToggle: vi.fn(),
	onCancel: vi.fn(),
} as const

function setup(overrides: Partial<Parameters<typeof HomeChatSlot>[0]> = {}) {
	const onSubmit = vi.fn()
	const onChange = vi.fn()
	render(
		<HomeChatSlot
			value="Ajoute un README"
			onChange={onChange}
			onSubmit={onSubmit}
			isPending={false}
			voice={VOICE}
			model={{ kind: 'ready', model: 'claude-sonnet-5-5' }}
			{...overrides}
		/>,
	)
	return { onSubmit, onChange }
}

describe('HomeChatSlot', () => {
	it('asks « Que lance-t-on ? » with the mic and the Captain model', () => {
		setup()

		expect(
			screen.getByRole('heading', { name: 'Que lance-t-on ?' }),
		).toBeTruthy()
		expect(screen.getByLabelText('Que lance-t-on ?')).toBeTruthy()
		expect(screen.getByRole('button', { name: 'Dicter' })).toBeTruthy()
		expect(screen.getByText('claude-sonnet-5-5')).toBeTruthy()
	})

	it('creates the conversation on Enter and on the send button', async () => {
		const { onSubmit } = setup()

		await userEvent.type(screen.getByLabelText('Que lance-t-on ?'), '{Enter}')
		await userEvent.click(screen.getByRole('button', { name: 'Envoyer' }))

		expect(onSubmit).toHaveBeenCalledTimes(2)
	})

	it('locks the field while the conversation is created and shows a failure', () => {
		setup({
			isPending: true,
			errorMessage: 'Les agents ne peuvent pas utiliser le chat.',
		})

		expect(
			(screen.getByLabelText('Que lance-t-on ?') as HTMLTextAreaElement)
				.disabled,
		).toBe(true)
		expect(screen.getByRole('alert').textContent).toBe(
			'Les agents ne peuvent pas utiliser le chat.',
		)
	})
})
