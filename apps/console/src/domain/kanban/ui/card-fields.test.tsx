import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { CardFields, EditableDescription } from '#/domain/kanban/ui/card-fields'
import { chooseValue } from '#/test-utils/combobox'

const MEMBERS = [
	{ subject: 'u1', name: 'Claire Martin', email: null, last_seen_at: 'x' },
	{ subject: 'u2', name: 'Bastien', email: null, last_seen_at: 'x' },
] as Schemas.MemberResponse[]

const CARD = {
	id: 'c1',
	version: 3,
	title: 'T',
	description: 'D',
	priority: 'normal',
	due_date: null,
	assignee_subject: null,
	estimate: null,
	labels: ['infra'],
	acceptance_criteria: [
		{ text: 'Les builds passent', done: false },
		{ text: 'Documenté', done: true },
	],
} as unknown as Schemas.CardResponse

function setup(card: Schemas.CardResponse = CARD, disabled = false) {
	const onEdit = vi.fn()
	render(
		<CardFields
			card={card}
			members={MEMBERS}
			membersReady
			disabled={disabled}
			onEdit={onEdit}
		/>,
	)
	return onEdit
}

describe('CardFields', () => {
	it('saves the priority and the estimate as soon as they change', async () => {
		const onEdit = setup()

		await chooseValue(
			userEvent.setup(),
			screen.getByLabelText('Priorité'),
			'urgent',
		)
		await chooseValue(
			userEvent.setup(),
			screen.getByLabelText('Estimation'),
			'xl',
		)

		expect(onEdit).toHaveBeenNthCalledWith(1, { priority: 'urgent' })
		expect(onEdit).toHaveBeenNthCalledWith(2, { estimate: 'xl' })
	})

	it('clears the estimate with null', async () => {
		const onEdit = setup({ ...CARD, estimate: 'm' })

		await chooseValue(
			userEvent.setup(),
			screen.getByLabelText('Estimation'),
			'',
		)

		expect(onEdit).toHaveBeenCalledWith({ estimate: null })
	})

	it('saves a due date and clears it with null', async () => {
		const onEdit = setup()
		const date = screen.getByLabelText('Échéance')

		await userEvent.type(date, '2026-10-31')
		expect(onEdit).toHaveBeenLastCalledWith({ due_date: '2026-10-31' })
	})

	it('clears the due date with null', async () => {
		const onEdit = setup({ ...CARD, due_date: '2026-10-31' })

		await userEvent.clear(screen.getByLabelText('Échéance'))

		expect(onEdit).toHaveBeenCalledWith({ due_date: null })
	})

	it('picks the responsable among the members', async () => {
		const onEdit = setup()
		const user = userEvent.setup()
		const select = screen.getByLabelText('Responsable')
		await user.click(select)
		const options = screen
			.getAllByRole('option')
			.map((option) => option.textContent)
		expect(options).toEqual(['Aucun', 'Claire Martin', 'Bastien'])

		await chooseValue(user, select, 'u2')
		expect(onEdit).toHaveBeenCalledWith({ assignee_subject: 'u2' })
	})

	it('clears the responsable with null', async () => {
		const onEdit = setup({ ...CARD, assignee_subject: 'u1' })

		await chooseValue(
			userEvent.setup(),
			screen.getByLabelText('Responsable'),
			'',
		)

		expect(onEdit).toHaveBeenCalledWith({ assignee_subject: null })
	})

	it('adds and removes a label', async () => {
		const onEdit = setup()

		await userEvent.type(screen.getByLabelText('Ajouter une étiquette'), ' ci ')
		await userEvent.click(
			screen.getByRole('button', { name: 'Ajouter l’étiquette' }),
		)
		expect(onEdit).toHaveBeenLastCalledWith({ labels: ['infra', 'ci'] })

		await userEvent.click(
			screen.getByRole('button', { name: 'Retirer l’étiquette infra' }),
		)
		expect(onEdit).toHaveBeenLastCalledWith({ labels: [] })
	})

	it('does not add a label that is already there', async () => {
		const onEdit = setup()

		await userEvent.type(
			screen.getByLabelText('Ajouter une étiquette'),
			'INFRA{Enter}',
		)

		expect(onEdit).not.toHaveBeenCalled()
	})

	it('checks a criterion', async () => {
		const onEdit = setup()

		await userEvent.click(
			screen.getByRole('checkbox', { name: 'Les builds passent' }),
		)

		expect(onEdit).toHaveBeenCalledWith({
			acceptance_criteria: [
				{ text: 'Les builds passent', done: true },
				{ text: 'Documenté', done: true },
			],
		})
	})

	it('edits the text of a criterion in place', async () => {
		const onEdit = setup()

		await userEvent.click(
			screen.getByRole('button', {
				name: 'Modifier le critère Documenté',
			}),
		)
		const field = screen.getByLabelText('Critère')
		await userEvent.clear(field)
		await userEvent.type(field, 'Documenté dans le README')
		await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))

		expect(onEdit).toHaveBeenCalledWith({
			acceptance_criteria: [
				{ text: 'Les builds passent', done: false },
				{ text: 'Documenté dans le README', done: true },
			],
		})
	})

	it('adds and removes a criterion', async () => {
		const onEdit = setup()

		await userEvent.type(screen.getByLabelText('Nouveau critère'), 'Livré')
		await userEvent.click(
			screen.getByRole('button', { name: 'Ajouter le critère' }),
		)
		expect(onEdit).toHaveBeenLastCalledWith({
			acceptance_criteria: [
				{ text: 'Les builds passent', done: false },
				{ text: 'Documenté', done: true },
				{ text: 'Livré', done: false },
			],
		})

		await userEvent.click(
			screen.getByRole('button', {
				name: 'Supprimer le critère Les builds passent',
			}),
		)
		expect(onEdit).toHaveBeenLastCalledWith({
			acceptance_criteria: [{ text: 'Documenté', done: true }],
		})
	})

	it('shows the progress of the criteria', () => {
		setup()
		expect(screen.getByText('1/2')).toBeTruthy()
	})

	it('locks every control while a save is running', () => {
		setup(CARD, true)

		expect(
			(screen.getByLabelText('Priorité') as HTMLSelectElement).disabled,
		).toBe(true)
		expect(
			(screen.getByRole('checkbox', { name: 'Documenté' }) as HTMLInputElement)
				.disabled,
		).toBe(true)
	})
})

describe('EditableDescription', () => {
	it('edits the description in place and saves it once', async () => {
		const onSave = vi.fn()
		render(
			<EditableDescription
				description="Ancienne"
				disabled={false}
				onSave={onSave}
			/>,
		)

		await userEvent.click(
			screen.getByRole('button', { name: 'Modifier la description' }),
		)
		const field = screen.getByLabelText('Description')
		await userEvent.clear(field)
		await userEvent.type(field, 'Nouvelle')
		await userEvent.click(screen.getByRole('button', { name: 'Enregistrer' }))

		expect(onSave).toHaveBeenCalledTimes(1)
		expect(onSave).toHaveBeenCalledWith('Nouvelle')
		expect(screen.queryByLabelText('Description')).toBeNull()
	})

	it('cancels without saving', async () => {
		const onSave = vi.fn()
		render(
			<EditableDescription
				description="Ancienne"
				disabled={false}
				onSave={onSave}
			/>,
		)

		await userEvent.click(
			screen.getByRole('button', { name: 'Modifier la description' }),
		)
		await userEvent.click(screen.getByRole('button', { name: 'Annuler' }))

		expect(onSave).not.toHaveBeenCalled()
		expect(screen.getByText('Ancienne')).toBeTruthy()
	})
})
