import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { DeleteGithubAppDialog } from '#/domain/integrations/ui/delete-github-app-dialog'

describe('DeleteGithubAppDialog', () => {
	it('explains that the App must also be deleted on GitHub, and confirms', async () => {
		const onConfirm = vi.fn()
		render(
			<DeleteGithubAppDialog
				open
				htmlUrl="https://github.com/apps/capi"
				isPending={false}
				onConfirm={onConfirm}
				onClose={vi.fn()}
			/>,
		)

		expect(screen.getByText(/L’application reste sur GitHub/)).toBeTruthy()
		expect(
			screen
				.getByRole('link', { name: 'ouvrir la page de l’application' })
				.getAttribute('href'),
		).toBe('https://github.com/apps/capi')
		await userEvent.click(
			screen.getByRole('button', { name: 'Supprimer l’application' }),
		)
		expect(onConfirm).toHaveBeenCalledOnce()
	})

	it('shows the failure and locks while pending', () => {
		render(
			<DeleteGithubAppDialog
				open
				isPending
				errorMessage="Impossible de supprimer l’application GitHub."
				onConfirm={vi.fn()}
				onClose={vi.fn()}
			/>,
		)

		expect(screen.getByRole('alert').textContent).toContain('Impossible')
		expect(
			(
				screen.getByRole('button', {
					name: 'Suppression…',
				}) as HTMLButtonElement
			).disabled,
		).toBe(true)
	})
})
