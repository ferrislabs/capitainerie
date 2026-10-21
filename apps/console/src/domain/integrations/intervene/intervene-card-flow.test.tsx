import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { InterveneCardFlow } from '#/domain/integrations/intervene/intervene-card-flow'
import {
	N8N_DEV,
	ODOO_OLD,
	ODOO_PROD,
	ODOO_STAGING,
	TODO_CARD,
} from '#/domain/integrations/intervene/test-fixtures'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

const LIST = '/api/v1/clients/cl1/integrations'
const INSTANCES = { items: [ODOO_PROD, ODOO_OLD, ODOO_STAGING, N8N_DEV] }

type Route = (init: RequestInit) => Response

describe('InterveneCardFlow', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function serve(table: Record<string, Route>) {
		fetchMock.mockImplementation((url: URL, init: RequestInit) => {
			const route = table[`${init.method} ${url.pathname}`]
			if (!route) throw new Error(`unexpected ${init.method} ${url.pathname}`)
			return Promise.resolve(route(init))
		})
	}

	function pick(name: string, choice: string) {
		return userEvent.click(
			within(screen.getByRole('radiogroup', { name })).getByRole('radio', {
				name: choice,
			}),
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

	it('lists the active instances of the client of the card', async () => {
		serve({ [`GET ${LIST}`]: () => json(200, INSTANCES) })
		render(
			<InterveneCardFlow
				card={TODO_CARD}
				onClose={vi.fn()}
				onStarted={vi.fn()}
			/>,
			{ wrapper: harness.wrapper },
		)

		expect(
			await screen.findByRole('radiogroup', { name: 'Odoo prod' }),
		).toBeTruthy()
		expect(screen.getAllByRole('radiogroup')).toHaveLength(3)
		expect(screen.queryByRole('radiogroup', { name: 'Odoo ancien' })).toBeNull()
		expect(
			screen.getByRole('radiogroup', { name: 'Odoo recette' }),
		).toBeTruthy()
		expect(screen.getByRole('radiogroup', { name: 'Flux' })).toBeTruthy()
	})

	it('sends the grants, the trimmed instructions, the confirmation and the card version, then reports the run', async () => {
		const bodies: Array<unknown> = []
		serve({
			[`GET ${LIST}`]: () => json(200, INSTANCES),
			'POST /api/v1/cards/c1/interventions': (init) => {
				bodies.push(JSON.parse(String(init.body)))
				return json(202, { card: TODO_CARD, run_id: 'run-9' })
			},
		})
		const onStarted = vi.fn()
		const user = userEvent.setup()
		render(
			<InterveneCardFlow
				card={TODO_CARD}
				onClose={vi.fn()}
				onStarted={onStarted}
			/>,
			{ wrapper: harness.wrapper },
		)
		await screen.findByRole('radiogroup', { name: 'Odoo prod' })

		await pick('Odoo prod', 'Écriture')
		await pick('Odoo recette', 'Lecture')
		await user.click(
			screen.getByLabelText('J’autorise l’écriture en production'),
		)
		await user.type(
			screen.getByLabelText('Instructions'),
			'  Mettre à jour le client  ',
		)
		await user.click(screen.getByRole('button', { name: 'Confier' }))

		await waitFor(() => expect(onStarted).toHaveBeenCalledWith('run-9'))
		expect(bodies).toEqual([
			{
				grants: [
					{ instance_id: 'i1', access: 'write' },
					{ instance_id: 'i2', access: 'read' },
				],
				instructions: 'Mettre à jour le client',
				confirm_production_writes: true,
				expected_version: 5,
			},
		])
	})

	it('shows the mapped message when the card already runs', async () => {
		serve({
			[`GET ${LIST}`]: () => json(200, INSTANCES),
			'POST /api/v1/cards/c1/interventions': () =>
				json(409, {
					code: 'conflict',
					status: 409,
					message: 'execution_already_active',
				}),
		})
		const onStarted = vi.fn()
		const user = userEvent.setup()
		render(
			<InterveneCardFlow
				card={TODO_CARD}
				onClose={vi.fn()}
				onStarted={onStarted}
			/>,
			{ wrapper: harness.wrapper },
		)
		await screen.findByRole('radiogroup', { name: 'Flux' })

		await pick('Flux', 'Lecture')
		await user.click(screen.getByRole('button', { name: 'Confier' }))

		expect(
			await screen.findByText(
				'Une exécution est déjà en cours pour cette carte.',
			),
		).toBeTruthy()
		expect(onStarted).not.toHaveBeenCalled()
	})

	it('shows the French message of a refusal code', async () => {
		serve({
			[`GET ${LIST}`]: () => json(200, INSTANCES),
			'POST /api/v1/cards/c1/interventions': () =>
				json(422, {
					code: 'instance_archived',
					status: 422,
					message: 'this instance is archived',
				}),
		})
		const user = userEvent.setup()
		render(
			<InterveneCardFlow
				card={TODO_CARD}
				onClose={vi.fn()}
				onStarted={vi.fn()}
			/>,
			{ wrapper: harness.wrapper },
		)
		await screen.findByRole('radiogroup', { name: 'Flux' })

		await pick('Flux', 'Lecture')
		await user.click(screen.getByRole('button', { name: 'Confier' }))

		expect(
			await screen.findByText(/Une instance choisie a été archivée/),
		).toBeTruthy()
	})

	it('offers a retry when the instances cannot be loaded', async () => {
		let calls = 0
		serve({
			[`GET ${LIST}`]: () => {
				calls += 1
				return calls === 1
					? json(500, { code: 'internal', status: 500, message: 'boom' })
					: json(200, INSTANCES)
			},
		})
		const user = userEvent.setup()
		render(
			<InterveneCardFlow
				card={TODO_CARD}
				onClose={vi.fn()}
				onStarted={vi.fn()}
			/>,
			{ wrapper: harness.wrapper },
		)

		await user.click(
			await screen.findByRole(
				'button',
				{ name: 'Réessayer' },
				{ timeout: 5000 },
			),
		)

		expect(await screen.findByRole('radiogroup', { name: 'Flux' })).toBeTruthy()
	}, 10000)

	it('closes without sending anything', async () => {
		serve({ [`GET ${LIST}`]: () => json(200, INSTANCES) })
		const onClose = vi.fn()
		render(
			<InterveneCardFlow
				card={TODO_CARD}
				onClose={onClose}
				onStarted={vi.fn()}
			/>,
			{ wrapper: harness.wrapper },
		)
		await screen.findByRole('radiogroup', { name: 'Flux' })

		await userEvent.click(screen.getByRole('button', { name: 'Annuler' }))

		expect(onClose).toHaveBeenCalledOnce()
		expect(
			fetchMock.mock.calls.some(
				([, init]) => (init as RequestInit).method === 'POST',
			),
		).toBe(false)
	})

	it('does not fetch while closed or for a card without client', () => {
		const { rerender } = render(
			<InterveneCardFlow
				card={undefined}
				onClose={vi.fn()}
				onStarted={vi.fn()}
			/>,
			{ wrapper: harness.wrapper },
		)
		rerender(
			<InterveneCardFlow
				card={{ ...TODO_CARD, client: null }}
				onClose={vi.fn()}
				onStarted={vi.fn()}
			/>,
		)

		expect(fetchMock).not.toHaveBeenCalled()
		expect(screen.queryByRole('dialog')).toBeNull()
	})
})
