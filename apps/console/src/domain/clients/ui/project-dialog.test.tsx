import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { ProjectDialog } from '#/domain/clients/ui/project-dialog'
import { chooseValue } from '#/test-utils/combobox'

const CLIENTS = [
	{ id: 'cl1', name: 'Acme' },
	{ id: 'cl2', name: 'Globex' },
] as Schemas.ClientResponse[]

function open(onSubmit = vi.fn()) {
	render(
		<ProjectDialog
			open
			clients={CLIENTS}
			isPending={false}
			onSubmit={onSubmit}
			onClose={() => {}}
		/>,
	)
	return onSubmit
}

describe('ProjectDialog', () => {
	it('submits the chosen client, and none for an internal project', async () => {
		const user = userEvent.setup()
		const onSubmit = open()
		await user.type(screen.getByLabelText('Nom'), 'Refonte')
		await chooseValue(user, screen.getByLabelText('Client'), 'cl2')
		expect(screen.getByLabelText('Client').textContent).toBe('Globex')
		await user.click(screen.getByRole('button', { name: 'Créer' }))
		expect(onSubmit).toHaveBeenLastCalledWith(
			expect.objectContaining({ name: 'Refonte', clientId: 'cl2' }),
		)

		await chooseValue(user, screen.getByLabelText('Client'), '')
		await user.click(screen.getByRole('button', { name: 'Créer' }))
		expect(onSubmit).toHaveBeenLastCalledWith(
			expect.objectContaining({ clientId: '' }),
		)
	})

	it('shows the name error after a first attempt, tied to the field', async () => {
		const user = userEvent.setup()
		open()
		await user.click(screen.getByRole('button', { name: 'Créer' }))
		const name = screen.getByLabelText('Nom')
		expect(name.getAttribute('aria-invalid')).toBe('true')
		expect(screen.getAllByRole('alert').length).toBeGreaterThan(0)
	})
})
