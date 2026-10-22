import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { RevertDialog } from '#/domain/integrations/journal/revert-dialog'

function action(
	overrides: Partial<Schemas.IntegrationActionResponse> = {},
): Schemas.IntegrationActionResponse {
	return {
		id: 'a1',
		instance_id: 'i1',
		instance_name: 'Odoo prod',
		instance_environment: 'production',
		instance_kind: 'odoo_json2',
		actor: 'capitainerie-agent-integrator',
		tool: 'odoo_write',
		access: 'write',
		target: { model: 'res.partner', ids: [2] },
		arguments: {},
		outcome: 'succeeded',
		revertible: true,
		created_at: '2026-10-16T09:31:00Z',
		...overrides,
	}
}

const CHECKBOX = 'J’autorise l’écriture en production'

function setup(overrides: Partial<Parameters<typeof RevertDialog>[0]> = {}) {
	const handlers = { onConfirm: vi.fn(), onClose: vi.fn() }
	render(
		<RevertDialog
			action={action()}
			isPending={false}
			{...handlers}
			{...overrides}
		/>,
	)
	return handlers
}

function confirm() {
	return screen.getByRole('button', { name: 'Restaurer' }) as HTMLButtonElement
}

describe('RevertDialog', () => {
	it('says what will be written back and where', () => {
		setup({ action: action({ instance_environment: 'staging' }) })

		expect(
			screen.getByRole('alertdialog', { name: 'Restaurer l’état d’avant ?' }),
		).toBeTruthy()
		expect(
			screen.getByText('L’état d’avant sera réécrit sur Odoo prod.'),
		).toBeTruthy()
		expect(screen.getByText('Odoo · modifier res.partner #2')).toBeTruthy()
	})

	it('confirms a non-production restoration straight away, without a checkbox', async () => {
		const handlers = setup({
			action: action({ instance_environment: 'staging' }),
		})
		expect(screen.queryByLabelText(CHECKBOX)).toBeNull()
		expect(confirm().disabled).toBe(false)

		await userEvent.click(confirm())

		expect(handlers.onConfirm).toHaveBeenCalledExactlyOnceWith(false)
	})

	it('holds a production restoration until the write is authorised', async () => {
		const handlers = setup()
		const checkbox = screen.getByLabelText(CHECKBOX) as HTMLInputElement
		expect(checkbox.required).toBe(true)
		expect(checkbox.checked).toBe(false)
		expect(confirm().disabled).toBe(true)

		await userEvent.click(confirm())
		expect(handlers.onConfirm).not.toHaveBeenCalled()

		await userEvent.click(checkbox)
		expect(confirm().disabled).toBe(false)
		await userEvent.click(confirm())

		expect(handlers.onConfirm).toHaveBeenCalledExactlyOnceWith(true)
	})

	it('shows the error and locks the form while pending', () => {
		setup({
			action: action({ instance_environment: 'staging' }),
			errorMessage: 'Cette action est déjà restaurée.',
			isPending: true,
		})

		expect(screen.getByRole('alert').textContent).toBe(
			'Cette action est déjà restaurée.',
		)
		expect(
			(
				screen.getByRole('button', {
					name: 'Restauration…',
				}) as HTMLButtonElement
			).disabled,
		).toBe(true)
		expect(
			(screen.getByRole('button', { name: 'Annuler' }) as HTMLButtonElement)
				.disabled,
		).toBe(true)
	})

	it('closes from the cancel button', async () => {
		const handlers = setup()

		await userEvent.click(screen.getByRole('button', { name: 'Annuler' }))

		expect(handlers.onClose).toHaveBeenCalledOnce()
	})
})
