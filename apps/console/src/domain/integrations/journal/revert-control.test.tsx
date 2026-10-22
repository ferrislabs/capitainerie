import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import type { Schemas } from '#/api/api.client'
import { RevertControl } from '#/domain/integrations/journal/revert-control'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

const REVERT = '/api/v1/integration-actions/a1/revert'
const CHECKBOX = 'J’autorise l’écriture en production'

function action(
	overrides: Partial<Schemas.IntegrationActionResponse> = {},
): Schemas.IntegrationActionResponse {
	return {
		id: 'a1',
		instance_id: 'i1',
		instance_name: 'Odoo recette',
		instance_environment: 'staging',
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

function restoreRow(
	overrides: Partial<Schemas.IntegrationActionResponse> = {},
): Schemas.IntegrationActionResponse {
	return action({
		id: 'r1',
		revertible: false,
		reverts_action_id: 'a1',
		...overrides,
	})
}

describe('RevertControl', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function respond(status: number, body: unknown) {
		fetchMock.mockImplementation(() => Promise.resolve(json(status, body)))
	}

	function trigger() {
		return screen.getByRole('button', {
			name: 'Restaurer l’état d’avant',
		}) as HTMLButtonElement
	}

	function setup(props: Partial<Parameters<typeof RevertControl>[0]> = {}) {
		const onRestored = vi.fn()
		render(
			<RevertControl
				action={action()}
				locked={false}
				onRestored={onRestored}
				{...props}
			/>,
			{ wrapper: harness.wrapper },
		)
		return { onRestored }
	}

	async function confirm() {
		await userEvent.click(trigger())
		await userEvent.click(
			await screen.findByRole('button', { name: 'Restaurer' }),
		)
	}

	beforeEach(() => {
		harness = createTestClient()
		api.setBaseUrl('http://api.test')
		vi.stubGlobal('fetch', fetchMock)
		setAuth({ accessToken: 'tok', isAuthenticated: true })
	})

	afterEach(() => {
		fetchMock.mockReset()
		vi.unstubAllGlobals()
		clearAuth()
		harness.client.clear()
	})

	it('is disabled with a reason while the intervention is still running', () => {
		setup({ locked: true })

		expect(trigger().disabled).toBe(true)
		expect(screen.getByText('L’intervention est encore en cours').id).toBe(
			trigger().getAttribute('aria-describedby'),
		)
	})

	it('opens the confirmation without sending anything yet', async () => {
		setup()
		expect(screen.queryByText('L’intervention est encore en cours')).toBeNull()

		await userEvent.click(trigger())

		expect(
			await screen.findByText('L’état d’avant sera réécrit sur Odoo recette.'),
		).toBeTruthy()
		expect(fetchMock).not.toHaveBeenCalled()
	})

	it('sends an empty body for a non-production instance, then reports the restoration', async () => {
		const bodies: Array<unknown> = []
		fetchMock.mockImplementation((_url: URL, init: RequestInit) => {
			bodies.push(JSON.parse(String(init.body)))
			return Promise.resolve(json(200, { items: [restoreRow()] }))
		})
		const { onRestored } = setup()

		await confirm()

		await waitFor(() => expect(onRestored).toHaveBeenCalledOnce())
		expect(bodies).toEqual([{}])
		expect(String((fetchMock.mock.calls[0] as [URL])[0])).toBe(
			`http://api.test${REVERT}`,
		)
		await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull())
	})

	it('sends confirm_production once a production write is authorised', async () => {
		const bodies: Array<unknown> = []
		fetchMock.mockImplementation((_url: URL, init: RequestInit) => {
			bodies.push(JSON.parse(String(init.body)))
			return Promise.resolve(json(200, { items: [restoreRow()] }))
		})
		const { onRestored } = setup({
			action: action({ instance_environment: 'production' }),
		})
		await userEvent.click(trigger())
		const submit = (await screen.findByRole('button', {
			name: 'Restaurer',
		})) as HTMLButtonElement
		expect(submit.disabled).toBe(true)

		await userEvent.click(screen.getByLabelText(CHECKBOX))
		await userEvent.click(submit)

		await waitFor(() => expect(onRestored).toHaveBeenCalledOnce())
		expect(bodies).toEqual([{ confirm_production: true }])
	})

	it('shows a restoration that stopped midway as a failure although the status is 200', async () => {
		respond(200, {
			items: [
				restoreRow(),
				restoreRow({
					id: 'r2',
					outcome: 'failed',
					reason_code: 'connection_failed',
					reason: 'timeout',
				}),
			],
		})
		const { onRestored } = setup()

		await confirm()

		expect(
			await screen.findByText(
				'La restauration s’est arrêtée : connexion à l’instance impossible',
			),
		).toBeTruthy()
		expect(onRestored).not.toHaveBeenCalled()
		expect(screen.getByRole('alertdialog')).toBeTruthy()
	})

	it.each([
		[
			409,
			'changed_since',
			'L’état a changé depuis cette action : rien n’a été restauré.',
		],
		[409, 'already_reverted', 'Cette action est déjà restaurée.'],
		[409, 'intervention_running', 'L’intervention est encore en cours.'],
		[422, 'not_revertible', 'Cette action ne peut pas être restaurée.'],
		[
			422,
			'instance_archived',
			'L’instance est archivée : l’état d’avant ne peut pas y être réécrit.',
		],
		[
			422,
			'production_write_not_confirmed',
			'Cochez l’autorisation d’écriture en production.',
		],
		[
			422,
			'connection_failed',
			'Connexion impossible : the instance did not answer in time',
		],
		[403, 'forbidden', 'Vous n’avez pas le droit de restaurer cette action.'],
	])('shows the French message of a %i %s', async (status, code, message) => {
		respond(status, {
			code,
			status,
			message:
				code === 'connection_failed'
					? 'the instance did not answer in time'
					: code,
		})
		const { onRestored } = setup()

		await confirm()

		expect((await screen.findByRole('alert')).textContent).toBe(message)
		expect(onRestored).not.toHaveBeenCalled()
	})

	it('closes the confirmation from the cancel button', async () => {
		setup()
		await userEvent.click(trigger())

		await userEvent.click(
			await screen.findByRole('button', { name: 'Annuler' }),
		)

		await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull())
		expect(fetchMock).not.toHaveBeenCalled()
	})
})
