import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import type { ReactNode } from 'react'
import { describe, expect, it, vi } from 'vitest'
import type { Schemas } from '#/api/api.client'
import { ProposalCard } from '#/domain/chat/ui/proposal-card'

vi.mock('@tanstack/react-router', () => ({
	Link: ({
		to,
		search,
		children,
	}: {
		to: string
		search?: { card?: string }
		children: ReactNode
	}) => <a href={`${to}?card=${search?.card}`}>{children}</a>,
}))

const PROPOSAL = { cardId: 'c1', title: 'Ajouter un README' }

function card(column: Schemas.ColumnDto): Schemas.CardResponse {
	return {
		id: 'c1',
		title: 'Ajouter un README',
		description: 'Dépôt test d’Acme',
		column,
		created_by_subject: 'ada',
		created_by_agent: 'capitainerie-agent-captain',
		version: 2,
		created_at: '2026-10-19T10:00:00Z',
		updated_at: '2026-10-19T10:00:00Z',
	}
}

function setup(overrides: Partial<Parameters<typeof ProposalCard>[0]> = {}) {
	const handlers = {
		onValidate: vi.fn(),
		onReject: vi.fn(),
		onExecute: vi.fn(),
	}
	render(
		<ProposalCard
			proposal={PROPOSAL}
			card={card('proposed')}
			pending={false}
			{...handlers}
			{...overrides}
		/>,
	)
	return handlers
}

describe('ProposalCard', () => {
	it('offers the four actions on a proposed card, each calling its handler', async () => {
		const handlers = setup()

		expect(screen.getByText('À valider')).toBeTruthy()
		await userEvent.click(screen.getByRole('button', { name: 'Valider' }))
		await userEvent.click(screen.getByRole('button', { name: 'Rejeter' }))
		await userEvent.click(
			screen.getByRole('button', { name: 'Confier à Claude Code' }),
		)

		expect(handlers.onValidate).toHaveBeenCalledWith(card('proposed'))
		expect(handlers.onReject).toHaveBeenCalledWith(card('proposed'))
		expect(handlers.onExecute).toHaveBeenCalledWith(card('proposed'))
		expect(
			screen.getByRole('link', { name: 'Ouvrir' }).getAttribute('href'),
		).toBe('/kanban?card=c1')
	})

	it('keeps only the execution and the link once the card is validated', () => {
		setup({ card: card('todo') })

		expect(screen.getByText('À faire')).toBeTruthy()
		expect(screen.queryByRole('button', { name: 'Valider' })).toBeNull()
		expect(screen.queryByRole('button', { name: 'Rejeter' })).toBeNull()
		expect(
			screen.getByRole('button', { name: 'Confier à Claude Code' }),
		).toBeTruthy()
	})

	it.each([
		['in_progress', 'En cours'],
		['done', 'Terminé'],
		['rejected', 'Rejeté'],
	] as const)('shows %s as a word and no action', (column, label) => {
		setup({ card: card(column) })

		expect(screen.getByText(label)).toBeTruthy()
		expect(screen.queryByRole('button')).toBeNull()
		expect(screen.getByRole('link', { name: 'Ouvrir' })).toBeTruthy()
	})

	it('disables the actions while the card moves and shows an error', () => {
		setup({ pending: true, errorMessage: 'La carte a changé, rechargée.' })

		expect(
			(screen.getByRole('button', { name: 'Valider' }) as HTMLButtonElement)
				.disabled,
		).toBe(true)
		expect(screen.getByRole('article').getAttribute('aria-busy')).toBe('true')
		expect(screen.getByRole('alert').textContent).toBe(
			'La carte a changé, rechargée.',
		)
	})

	it('shows only the title and the link while the card is unknown', () => {
		setup({ card: undefined })

		expect(screen.getByText('Ajouter un README')).toBeTruthy()
		expect(screen.queryByRole('button')).toBeNull()
		expect(screen.getByRole('link', { name: 'Ouvrir' })).toBeTruthy()
	})
})
