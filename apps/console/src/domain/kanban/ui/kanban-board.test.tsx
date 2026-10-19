import { render, screen } from '@testing-library/react'
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
		version: 1,
		created_at: '2026-10-16T09:30:00Z',
		updated_at: '2026-10-16T09:30:00Z',
	}
}

function board(cards: Schemas.CardResponse[]) {
	return render(
		<KanbanBoard
			cards={cards}
			pendingCardId={undefined}
			expandedIds={new Set()}
			onToggleExpanded={vi.fn()}
			onMove={vi.fn()}
			onOpen={vi.fn()}
		/>,
	)
}

describe('KanbanBoard', () => {
	it('renders the five columns in order with the proposed count', () => {
		board([card('1', 'proposed'), card('2', 'proposed'), card('3', 'done')])

		const headings = screen
			.getAllByRole('heading', { level: 2 })
			.map((h) => h.textContent)
		expect(headings).toEqual([
			'Proposé',
			'À faire',
			'En cours',
			'Terminé',
			'Rejeté',
		])
		expect(screen.getByTestId('proposed-count').textContent).toBe('2')
		expect(screen.getByText('Carte 3')).toBeTruthy()
	})

	it('renders the empty state without cards', () => {
		board([])

		expect(
			screen.getByText(
				'Aucune carte. Déposez un transcript : les agents proposeront des cartes ici.',
			),
		).toBeTruthy()
		expect(screen.queryByText('Proposé')).toBeNull()
	})
})
