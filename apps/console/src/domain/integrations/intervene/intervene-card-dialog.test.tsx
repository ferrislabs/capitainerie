import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import {
	type InstancesState,
	InterveneCardDialog,
} from '#/domain/integrations/intervene/intervene-card-dialog'
import {
	N8N_DEV,
	ODOO_PROD,
	ODOO_STAGING,
} from '#/domain/integrations/intervene/test-fixtures'

const READY: InstancesState = {
	kind: 'ready',
	instances: [ODOO_PROD, ODOO_STAGING, N8N_DEV],
}

function setup(
	overrides: Partial<Parameters<typeof InterveneCardDialog>[0]> = {},
) {
	const handlers = {
		onRetry: vi.fn(),
		onSubmit: vi.fn(),
		onClose: vi.fn(),
	}
	render(
		<InterveneCardDialog
			open
			cardTitle="Corriger le bug"
			instances={READY}
			isPending={false}
			{...handlers}
			{...overrides}
		/>,
	)
	return handlers
}

function group(name: string) {
	return screen.getByRole('radiogroup', { name })
}

function pick(name: string, choice: 'Aucun' | 'Lecture' | 'Écriture') {
	return userEvent.click(
		within(group(name)).getByRole('radio', { name: choice }),
	)
}

function submit() {
	return screen.getByRole('button', { name: 'Confier' }) as HTMLButtonElement
}

const CHECKBOX = 'J’autorise l’écriture en production'

describe('InterveneCardDialog', () => {
	it('shows one row per instance with its type and environment, defaulting to Aucun', () => {
		setup()

		expect(
			screen.getByRole('heading', { name: 'Confier à l’Intervenant' }),
		).toBeTruthy()
		const items = screen.getAllByRole('listitem')
		expect(items).toHaveLength(3)
		expect(within(items[0]).getByText('Odoo prod')).toBeTruthy()
		expect(within(items[0]).getByText('Odoo 19 et plus')).toBeTruthy()
		expect(within(items[0]).getByText('Production')).toBeTruthy()
		expect(within(items[1]).getByText('Odoo 18 et moins')).toBeTruthy()
		expect(within(items[1]).getByText('Staging')).toBeTruthy()
		expect(within(items[2]).getByText('n8n')).toBeTruthy()
		expect(within(items[2]).getByText('Développement')).toBeTruthy()
		for (const item of items) {
			const radios = within(item).getAllByRole(
				'radio',
			) as Array<HTMLInputElement>
			expect(radios.map((radio) => radio.labels?.[0]?.textContent)).toEqual([
				'Aucun',
				'Lecture',
				'Écriture',
			])
			expect(radios.map((radio) => radio.checked)).toEqual([true, false, false])
		}
	})

	it('needs one instance above Aucun to submit', async () => {
		const handlers = setup()
		expect(submit().disabled).toBe(true)

		await pick('Odoo recette', 'Lecture')
		expect(submit().disabled).toBe(false)

		await pick('Odoo recette', 'Aucun')
		expect(submit().disabled).toBe(true)

		await pick('Flux', 'Écriture')
		await userEvent.click(submit())

		expect(handlers.onSubmit).toHaveBeenCalledWith({
			choices: { i2: 'none', i3: 'write' },
			instructions: '',
			confirmed: false,
		})
	})

	it('asks for the production confirmation only for a production instance in write', async () => {
		const handlers = setup()
		expect(screen.queryByLabelText(CHECKBOX)).toBeNull()

		await pick('Odoo prod', 'Lecture')
		await pick('Odoo recette', 'Écriture')
		expect(screen.queryByLabelText(CHECKBOX)).toBeNull()
		expect(submit().disabled).toBe(false)

		await pick('Odoo prod', 'Écriture')
		const checkbox = screen.getByLabelText(CHECKBOX) as HTMLInputElement
		expect(checkbox.checked).toBe(false)
		expect(checkbox.required).toBe(true)
		expect(submit().disabled).toBe(true)

		await userEvent.click(checkbox)
		expect(checkbox.checked).toBe(true)
		expect(submit().disabled).toBe(false)

		await userEvent.click(submit())
		expect(handlers.onSubmit).toHaveBeenCalledWith({
			choices: { i1: 'write', i2: 'write' },
			instructions: '',
			confirmed: true,
		})
	})

	it('forgets the confirmation once no production instance is in write', async () => {
		setup()
		await pick('Odoo prod', 'Écriture')
		await userEvent.click(screen.getByLabelText(CHECKBOX))

		await pick('Odoo prod', 'Lecture')
		expect(screen.queryByLabelText(CHECKBOX)).toBeNull()

		await pick('Odoo prod', 'Écriture')
		expect((screen.getByLabelText(CHECKBOX) as HTMLInputElement).checked).toBe(
			false,
		)
		expect(submit().disabled).toBe(true)
	})

	it('takes optional instructions with a counter and a hint', async () => {
		const handlers = setup()
		const field = screen.getByLabelText('Instructions') as HTMLTextAreaElement
		expect(field.placeholder).toBe(
			'Par exemple : vérifier dans Odoo staging, ne rien modifier en production',
		)
		expect(screen.getByText('0 / 4000')).toBeTruthy()

		await userEvent.type(field, 'Vérifier le client')
		expect(screen.getByText('18 / 4000')).toBeTruthy()
		await pick('Odoo recette', 'Lecture')
		await userEvent.click(submit())

		expect(handlers.onSubmit).toHaveBeenCalledWith({
			choices: { i2: 'read' },
			instructions: 'Vérifier le client',
			confirmed: false,
		})
	})

	it('blocks instructions past 4 000 characters', async () => {
		setup()
		await pick('Odoo recette', 'Lecture')

		const field = screen.getByLabelText('Instructions') as HTMLTextAreaElement
		await userEvent.click(field)
		await userEvent.paste('x'.repeat(4001))

		expect(screen.getByText('4001 / 4000')).toBeTruthy()
		expect(submit().disabled).toBe(true)
	})

	it('always warns that the Intervenant acts within the grants and that each action is journaled', () => {
		setup()

		expect(
			screen.getByText(
				'L’Intervenant lit et modifie les instances choisies, dans la limite de ces autorisations. Chaque action est journalisée.',
			),
		).toBeTruthy()
	})

	it('shows the mapped error and locks the form while pending', async () => {
		setup({
			errorMessage: 'Une exécution est déjà en cours pour cette carte.',
			isPending: true,
		})

		expect(screen.getByRole('alert').textContent).toBe(
			'Une exécution est déjà en cours pour cette carte.',
		)
		expect(
			(screen.getByRole('button', { name: 'Envoi…' }) as HTMLButtonElement)
				.disabled,
		).toBe(true)
		for (const radio of screen.getAllByRole(
			'radio',
		) as Array<HTMLInputElement>) {
			expect(radio.disabled).toBe(true)
		}
		expect(
			(screen.getByLabelText('Instructions') as HTMLTextAreaElement).disabled,
		).toBe(true)
	})

	it('shows a loading state', () => {
		setup({ instances: { kind: 'loading' } })

		expect(screen.getByText('Chargement des intégrations…')).toBeTruthy()
		expect(screen.queryByRole('radio')).toBeNull()
		expect(submit().disabled).toBe(true)
	})

	it('offers a retry on a loading error', async () => {
		const handlers = setup({ instances: { kind: 'error' } })

		expect(screen.getByRole('alert').textContent).toBe(
			'Impossible de charger les intégrations du client.',
		)
		await userEvent.click(screen.getByRole('button', { name: 'Réessayer' }))
		expect(handlers.onRetry).toHaveBeenCalledOnce()
		expect(submit().disabled).toBe(true)
	})

	it('says when there is nothing to choose', () => {
		setup({ instances: { kind: 'ready', instances: [] } })

		expect(screen.getByText('Ce client n’a aucune intégration.')).toBeTruthy()
		expect(screen.queryByRole('radio')).toBeNull()
		expect(submit().disabled).toBe(true)
	})

	it('closes from the cancel button', async () => {
		const handlers = setup()

		await userEvent.click(screen.getByRole('button', { name: 'Annuler' }))

		expect(handlers.onClose).toHaveBeenCalledOnce()
	})
})
