import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { KanbanBoard } from '#/domain/kanban/ui/kanban-board'

function card(id: string, column: Schemas.ColumnDto): Schemas.CardResponse {
	return {
		id,
		title: `Carte ${id}`,
		description: '',
		column,
		provenance: null,
		created_by_run: null,
		created_by_subject: 'ada',
		created_by_agent: null,
		version: 2,
		created_at: '2026-10-16T09:30:00Z',
		updated_at: '2026-10-16T09:30:00Z',
		acceptance_criteria: [],
		priority: 'normal',
		labels: [],
	}
}

function setup() {
	const onOpen = vi.fn()
	const onMove = vi.fn()
	render(
		<KanbanBoard
			cards={[card('1', 'proposed'), card('2', 'todo')]}
			pendingCardId={undefined}
			expandedIds={new Set()}
			onToggleExpanded={vi.fn()}
			onMove={onMove}
			onOpen={onOpen}
		/>,
	)
	return { onOpen, onMove }
}

function cardElement(title: string) {
	return screen.getByRole('group', { name: title })
}

describe('KanbanBoard interactions', () => {
	it('opens a card on click', async () => {
		const { onOpen } = setup()

		await userEvent.click(screen.getByRole('heading', { name: 'Carte 2' }))

		expect(onOpen).toHaveBeenCalledWith('2')
	})

	it('opens a card on Enter and leaves the buttons of the card alone', async () => {
		const { onOpen, onMove } = setup()

		cardElement('Carte 2').focus()
		await userEvent.keyboard('{Enter}')
		expect(onOpen).toHaveBeenCalledWith('2')

		onOpen.mockClear()
		await userEvent.click(screen.getByRole('button', { name: 'Valider' }))
		expect(onOpen).not.toHaveBeenCalled()
		expect(onMove).toHaveBeenCalledWith('1', 'todo', 2)
	})

	it('announces the keyboard pick-up and cancel in French', async () => {
		const { onMove } = setup()

		cardElement('Carte 2').focus()
		await userEvent.keyboard(' ')
		const status = await screen.findByRole('status')
		await waitFor(() =>
			expect(status.textContent).toMatch(/^Carte « Carte 2 »/),
		)

		await userEvent.keyboard('{Escape}')
		await waitFor(() =>
			expect(status.textContent).toBe(
				'Déplacement annulé. Carte « Carte 2 » remise dans la colonne À faire.',
			),
		)
		expect(onMove).not.toHaveBeenCalled()
	})

	it('does not start a drag from Space or Enter on a button of the card', async () => {
		const { onMove, onOpen } = setup()

		screen.getByRole('button', { name: 'Valider' }).focus()
		await userEvent.keyboard(' ')

		expect(screen.queryByRole('status')?.textContent ?? '').not.toMatch(/prise/)
		expect(onOpen).not.toHaveBeenCalled()
		expect(onMove).toHaveBeenCalledTimes(1)
		expect(onMove).toHaveBeenCalledWith('1', 'todo', 2)
	})

	it('describes the gesture to screen readers', () => {
		setup()

		expect(cardElement('Carte 2').getAttribute('aria-roledescription')).toBe(
			'carte déplaçable',
		)
		expect(document.body.textContent?.includes('appuyez sur Espace')).toBe(true)
	})
})
