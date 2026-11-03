import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { api } from '#/api'
import { InterveneButton } from '#/domain/integrations/intervene/intervene-button'
import {
	N8N_DEV,
	ODOO_OLD,
	TODO_CARD,
} from '#/domain/integrations/intervene/test-fixtures'
import { createTestClient, json } from '#/domain/settings/hooks/test-utils'
import { clearAuth, setAuth } from '#/store/auth.store'

const NAME = 'Confier à l’Intervenant'

describe('InterveneButton', () => {
	const fetchMock = vi.fn()
	let harness: ReturnType<typeof createTestClient>

	function serve(status: number, body: unknown) {
		fetchMock.mockImplementation(() => Promise.resolve(json(status, body)))
	}

	function button() {
		return screen.getByRole('button', { name: NAME }) as HTMLButtonElement
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

	it('is disabled with a reason when the card has no client', () => {
		render(
			<InterveneButton
				card={{ ...TODO_CARD, client: null }}
				onIntervene={vi.fn()}
			/>,
			{ wrapper: harness.wrapper },
		)

		expect(button().disabled).toBe(true)
		expect(
			screen.getByText('Rattachez d’abord la carte à un client'),
		).toBeTruthy()
		expect(button().getAttribute('aria-describedby')).toBe(
			screen.getByText('Rattachez d’abord la carte à un client').id,
		)
		expect(fetchMock).not.toHaveBeenCalled()
	})

	it('is disabled with a reason when the client has no active instance', async () => {
		serve(200, { items: [ODOO_OLD] })
		render(<InterveneButton card={TODO_CARD} onIntervene={vi.fn()} />, {
			wrapper: harness.wrapper,
		})

		expect(
			await screen.findByText('Ce client n’a aucune intégration'),
		).toBeTruthy()
		expect(button().disabled).toBe(true)
	})

	it('waits for the instances without giving a reason', () => {
		fetchMock.mockImplementation(() => new Promise(() => undefined))
		render(<InterveneButton card={TODO_CARD} onIntervene={vi.fn()} />, {
			wrapper: harness.wrapper,
		})

		expect(button().disabled).toBe(true)
		expect(screen.queryByText(/Rattachez|aucune intégration/)).toBeNull()
	})

	it('hands the card over when the client has an active instance', async () => {
		serve(200, { items: [ODOO_OLD, N8N_DEV] })
		const onIntervene = vi.fn()
		render(<InterveneButton card={TODO_CARD} onIntervene={onIntervene} />, {
			wrapper: harness.wrapper,
		})

		await vi.waitFor(() => expect(button().disabled).toBe(false))
		await userEvent.click(button())

		expect(onIntervene).toHaveBeenCalledWith(TODO_CARD)
		expect(screen.queryByText('Ce client n’a aucune intégration')).toBeNull()
	})

	it('stays available when the instances cannot be read, so the dialog can say so', async () => {
		serve(500, { code: 'internal', status: 500, message: 'boom' })
		render(<InterveneButton card={TODO_CARD} onIntervene={vi.fn()} />, {
			wrapper: harness.wrapper,
		})

		await vi.waitFor(() => expect(button().disabled).toBe(false))
	})

	it('can be locked from outside', async () => {
		serve(200, { items: [N8N_DEV] })
		render(
			<InterveneButton card={TODO_CARD} disabled onIntervene={vi.fn()} />,
			{ wrapper: harness.wrapper },
		)

		await vi.waitFor(() => expect(fetchMock).toHaveBeenCalled())
		expect(button().disabled).toBe(true)
	})
})
