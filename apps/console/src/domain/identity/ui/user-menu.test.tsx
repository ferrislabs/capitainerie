import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { initials, UserMenu } from '#/domain/identity/ui/user-menu'

describe('UserMenu', () => {
	it('derives initials from the display name', () => {
		expect(initials('Ada Lovelace')).toBe('AL')
		expect(initials('ada')).toBe('AD')
		expect(initials('  ')).toBe('?')
	})

	it('opens with the profile and signs out', async () => {
		const onSignOut = vi.fn()
		render(
			<UserMenu
				displayName="Ada Lovelace"
				email="ada@example.com"
				organizationName="Acme"
				onSignOut={onSignOut}
			/>,
		)
		const user = userEvent.setup()

		const trigger = screen.getByRole('button', {
			name: 'Compte de Ada Lovelace',
		})
		trigger.focus()
		await user.keyboard('{Enter}')

		expect(await screen.findByText('ada@example.com')).toBeTruthy()
		await user.click(screen.getByRole('menuitem', { name: 'Se déconnecter' }))
		expect(onSignOut).toHaveBeenCalledOnce()
	})
})
