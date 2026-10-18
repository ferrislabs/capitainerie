import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { initials, UserMenu } from '#/domain/identity/ui/user-menu'

function renderMenu() {
	const onSignOut = vi.fn()
	const onThemeChange = vi.fn()
	render(
		<UserMenu
			displayName="Ada Lovelace"
			email="ada@example.com"
			organizationName="Acme"
			theme="system"
			onThemeChange={onThemeChange}
			onSignOut={onSignOut}
		/>,
	)
	return { onSignOut, onThemeChange, user: userEvent.setup() }
}

async function open(user: ReturnType<typeof userEvent.setup>) {
	const trigger = screen.getByRole('button', {
		name: 'Compte de Ada Lovelace',
	})
	trigger.focus()
	await user.keyboard('{Enter}')
}

describe('UserMenu', () => {
	it('derives initials from the display name', () => {
		expect(initials('Ada Lovelace')).toBe('AL')
		expect(initials('ada')).toBe('AD')
		expect(initials('  ')).toBe('?')
	})

	it('shows the organization and the user on the trigger', () => {
		renderMenu()

		expect(screen.getByText('Acme')).toBeTruthy()
		expect(screen.getByText('Ada Lovelace')).toBeTruthy()
	})

	it('opens with the profile and signs out', async () => {
		const { onSignOut, user } = renderMenu()
		await open(user)

		expect(await screen.findByText('ada@example.com')).toBeTruthy()
		await user.click(screen.getByRole('menuitem', { name: 'Se déconnecter' }))
		expect(onSignOut).toHaveBeenCalledOnce()
	})

	it('marks the current theme and changes it', async () => {
		const { onThemeChange, user } = renderMenu()
		await open(user)

		expect(
			(
				await screen.findByRole('menuitemradio', { name: 'Comme le système' })
			).getAttribute('aria-checked'),
		).toBe('true')
		await user.click(screen.getByRole('menuitemradio', { name: 'Sombre' }))
		expect(onThemeChange).toHaveBeenCalledWith('dark')
	})
})
