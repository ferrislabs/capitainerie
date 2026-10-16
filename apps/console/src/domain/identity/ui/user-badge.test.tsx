import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { UserBadge } from '#/domain/identity/ui/user-badge'

describe('UserBadge', () => {
	it('renders name, email and organization', () => {
		render(
			<UserBadge
				displayName="Ada Lovelace"
				email="ada@example.com"
				organizationName="Acme"
			/>,
		)

		expect(screen.getByText('Ada Lovelace')).toBeTruthy()
		expect(screen.getByText('ada@example.com')).toBeTruthy()
		expect(screen.getByText('Acme')).toBeTruthy()
	})

	it('omits the email when absent', () => {
		render(<UserBadge displayName="Ada" organizationName="Acme" />)

		expect(screen.queryByText(/@/)).toBeNull()
	})
})
