import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import {
	GithubAppForm,
	isValidOwner,
} from '#/domain/integrations/ui/github-app-form'

function setup(overrides: Partial<Parameters<typeof GithubAppForm>[0]> = {}) {
	const handlers = {
		onOwnerKindChange: vi.fn(),
		onLoginChange: vi.fn(),
		onSubmit: vi.fn(),
	}
	render(
		<GithubAppForm
			ownerKind="user"
			login=""
			isPending={false}
			{...handlers}
			{...overrides}
		/>,
	)
	return handlers
}

describe('GithubAppForm', () => {
	it('offers a personal account or a GitHub organization, personal by default', async () => {
		const handlers = setup()

		expect(
			screen.getByLabelText('Compte personnel').getAttribute('aria-checked'),
		).toBe('true')
		expect(screen.queryByLabelText(/Login de l’organisation/)).toBeNull()
		await userEvent.click(screen.getByLabelText('Organisation GitHub'))
		expect(handlers.onOwnerKindChange).toHaveBeenCalledWith('organization')
	})

	it('asks for the organization login and blocks an invalid one', async () => {
		const handlers = setup({ ownerKind: 'organization', login: 'a/b' })

		const button = screen.getByRole('button', {
			name: 'Créer l’application GitHub',
		}) as HTMLButtonElement
		expect(button.disabled).toBe(true)
		await userEvent.type(screen.getByLabelText(/Login de l’organisation/), 'x')
		expect(handlers.onLoginChange).toHaveBeenCalledWith('a/bx')
		expect(handlers.onSubmit).not.toHaveBeenCalled()
	})

	it('submits with a valid organization login', async () => {
		const handlers = setup({ ownerKind: 'organization', login: 'acme-corp' })

		await userEvent.click(
			screen.getByRole('button', { name: 'Créer l’application GitHub' }),
		)

		expect(handlers.onSubmit).toHaveBeenCalledOnce()
	})

	it('locks the form while the manifest is prepared and shows an error', () => {
		setup({ isPending: true, errorMessage: 'Échec.' })

		expect(
			(
				screen.getByRole('button', {
					name: 'Préparation…',
				}) as HTMLButtonElement
			).disabled,
		).toBe(true)
		expect(screen.getByRole('alert').textContent).toBe('Échec.')
	})

	it('validates owners', () => {
		expect(isValidOwner('user', '')).toBe(true)
		expect(isValidOwner('organization', ' acme ')).toBe(true)
		for (const bad of ['', 'a b', 'a/b', '-acme', 'a?b']) {
			expect(isValidOwner('organization', bad)).toBe(false)
		}
	})
})
