import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { CardItem } from '#/domain/kanban/ui/card-item'

function card(
	overrides: Partial<Schemas.CardResponse> = {},
): Schemas.CardResponse {
	return {
		id: 'c1',
		title: 'Écrire le compte rendu',
		description: 'Détails',
		column: 'proposed',
		provenance: {
			transcript_id: 't1',
			start: 0,
			end: 10,
			quote: 'il faut écrire',
		},
		created_by_run: null,
		created_by_subject: 'agent',
		created_by_agent: 'capitainerie-agent-writer',
		version: 4,
		created_at: new Date(Date.now() - 5 * 60_000).toISOString(),
		updated_at: new Date().toISOString(),
		...overrides,
	}
}

function setup(c: Schemas.CardResponse, isPending = false) {
	const onMove = vi.fn()
	const onToggleExpanded = vi.fn()
	const onExecute = vi.fn()
	render(
		<CardItem
			card={c}
			isPending={isPending}
			expanded={false}
			onToggleExpanded={onToggleExpanded}
			onMove={onMove}
			onExecute={onExecute}
		/>,
	)
	return { onMove, onToggleExpanded, onExecute }
}

describe('CardItem', () => {
	it('lays the card out in title, meta and footer zones', () => {
		const { container } = render(
			<CardItem
				card={card()}
				isPending={false}
				expanded={false}
				onToggleExpanded={vi.fn()}
				onMove={vi.fn()}
			/>,
		)

		const zones = [...container.querySelectorAll('[data-zone]')].map((zone) =>
			zone.getAttribute('data-zone'),
		)
		expect(zones).toEqual(['title', 'meta', 'footer'])
		const title = container.querySelector('[data-zone="title"]')
		expect(title?.querySelector('h3')?.textContent).toBe(
			'Écrire le compte rendu',
		)
		const meta = container.querySelector('[data-zone="meta"]')
		expect(meta?.textContent).toContain('il faut écrire')
		expect(meta?.textContent).toContain('writer')
		const footer = container.querySelector('[data-zone="footer"]')
		expect(footer?.textContent).toContain('Valider')
	})

	it('leaves the footer out when a card has no action', () => {
		const { container } = render(
			<CardItem
				card={card({ column: 'done' })}
				isPending={false}
				expanded={false}
				onToggleExpanded={vi.fn()}
				onMove={vi.fn()}
			/>,
		)

		expect(container.querySelector('[data-zone="footer"]')).toBeNull()
	})

	it('shows agent badge without prefix and the quote', () => {
		setup(card())

		expect(screen.getByText('writer')).toBeTruthy()
		expect(screen.getByText('« il faut écrire »')).toBeTruthy()
		expect(screen.getByText('extrait du transcript')).toBeTruthy()
		expect(screen.getByText('il y a 5 minutes')).toBeTruthy()
	})

	it('hides agent badge and quote when absent', () => {
		setup(card({ created_by_agent: null, provenance: null }))

		expect(screen.queryByText('writer')).toBeNull()
		expect(screen.queryByText('extrait du transcript')).toBeNull()
	})

	it('validates and rejects a proposed card with expected_version', async () => {
		const { onMove } = setup(card())

		await userEvent.click(screen.getByRole('button', { name: 'Valider' }))
		await userEvent.click(screen.getByRole('button', { name: 'Rejeter' }))

		expect(onMove).toHaveBeenNthCalledWith(1, 'c1', 'todo', 4)
		expect(onMove).toHaveBeenNthCalledWith(2, 'c1', 'rejected', 4)
	})

	it('leaves the move list to the card panel', () => {
		setup(card({ column: 'todo' }))

		expect(screen.queryByRole('button', { name: 'Valider' })).toBeNull()
		expect(screen.queryByLabelText('Déplacer vers…')).toBeNull()
	})

	it('disables actions while pending', () => {
		setup(card(), true)

		expect(
			(screen.getByRole('button', { name: 'Valider' }) as HTMLButtonElement)
				.disabled,
		).toBe(true)
	})

	it('toggles description expansion', async () => {
		const { onToggleExpanded } = setup(card())

		await userEvent.click(screen.getByRole('button', { name: 'Développer' }))

		expect(onToggleExpanded).toHaveBeenCalledWith('c1')
	})

	it('offers to hand a todo card to Claude Code', async () => {
		const todo = card({ column: 'todo' })
		const { onExecute } = setup(todo)

		await userEvent.click(
			screen.getByRole('button', { name: 'Confier à Claude Code' }),
		)

		expect(onExecute).toHaveBeenCalledWith(todo)
	})

	it('offers it on todo cards only', () => {
		for (const column of [
			'proposed',
			'in_progress',
			'done',
			'rejected',
		] as const) {
			const { unmount } = render(
				<CardItem
					card={card({ column })}
					isPending={false}
					expanded={false}
					onToggleExpanded={vi.fn()}
					onMove={vi.fn()}
					onExecute={vi.fn()}
				/>,
			)
			expect(
				screen.queryByRole('button', { name: 'Confier à Claude Code' }),
			).toBeNull()
			unmount()
		}
	})
})
