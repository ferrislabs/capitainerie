import { render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { CardSignals } from '#/domain/kanban/ui/card-signals'

const NOW = new Date(2026, 9, 19, 12, 0)

function card(
	overrides: Partial<Schemas.CardResponse> = {},
): Schemas.CardResponse {
	return {
		id: 'c1',
		title: 'T',
		description: '',
		column: 'todo',
		provenance: null,
		created_by_run: null,
		created_by_subject: 'ada',
		created_by_agent: null,
		version: 1,
		created_at: '2026-10-16T09:30:00Z',
		updated_at: '2026-10-16T09:30:00Z',
		acceptance_criteria: [],
		priority: 'normal',
		labels: [],
		...overrides,
	}
}

function show(c: Schemas.CardResponse, assigneeName?: string) {
	return render(<CardSignals card={c} assigneeName={assigneeName} now={NOW} />)
}

describe('CardSignals', () => {
	it('shows nothing for a card without details', () => {
		const { container } = show(card())
		expect(container.firstChild).toBeNull()
	})

	it('badges urgent and high priorities with a word and an icon, and no other', () => {
		const { container, unmount } = show(card({ priority: 'urgent' }))
		expect(screen.getByText('Urgente')).toBeTruthy()
		expect(container.querySelector('svg')).not.toBeNull()
		unmount()

		const high = show(card({ priority: 'high' }))
		expect(screen.getByText('Haute')).toBeTruthy()
		high.unmount()

		for (const priority of ['normal', 'low'] as const) {
			const other = show(card({ priority }))
			expect(other.container.querySelector('[data-priority]')).toBeNull()
			expect(screen.queryByText('Basse')).toBeNull()
			expect(screen.queryByText('Normale')).toBeNull()
			other.unmount()
		}
	})

	it('shows a past due date in red with « en retard »', () => {
		const { container } = show(card({ due_date: '2026-10-18' }))
		expect(screen.getByText('· en retard')).toBeTruthy()
		expect(container.querySelector('[data-overdue]')).not.toBeNull()
		expect(container.querySelector('[data-overdue]')?.className).toContain(
			'text-signal-danger',
		)
	})

	it('shows a due date today or later without the late mark', () => {
		for (const due_date of ['2026-10-19', '2026-11-02']) {
			const { container, unmount } = show(card({ due_date }))
			expect(screen.queryByText('· en retard')).toBeNull()
			expect(container.querySelector('[data-overdue]')).toBeNull()
			expect(container.querySelector('time')?.getAttribute('datetime')).toBe(
				due_date,
			)
			unmount()
		}
	})

	it('shows the initials of the assignee', () => {
		show(card({ assignee_subject: 'u1' }), 'Claire Martin')
		const badge = screen.getByRole('img', {
			name: 'Responsable : Claire Martin',
		})
		expect(badge.textContent).toBe('CM')
	})

	it('shows two labels then a +n counter', () => {
		show(card({ labels: ['infra', 'ci', 'doc', 'api'] }))
		expect(screen.getByText('infra')).toBeTruthy()
		expect(screen.getByText('ci')).toBeTruthy()
		expect(screen.queryByText('doc')).toBeNull()
		expect(screen.getByText('+2').getAttribute('title')).toBe('doc, api')
	})

	it('shows no counter when the labels fit', () => {
		show(card({ labels: ['infra', 'ci'] }))
		expect(screen.queryByText(/^\+/)).toBeNull()
	})

	it('shows the criteria progress as n/m', () => {
		show(
			card({
				acceptance_criteria: [
					{ text: 'a', done: true },
					{ text: 'b', done: false },
					{ text: 'c', done: false },
				],
			}),
		)
		expect(screen.getByText('1/3')).toBeTruthy()
	})
})
