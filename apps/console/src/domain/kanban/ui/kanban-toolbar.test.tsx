import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { NO_FILTERS } from '#/domain/kanban/card-filters'
import { KanbanToolbar } from '#/domain/kanban/ui/kanban-toolbar'

const OPTIONS = {
	clients: ['Acme', 'Globex'],
	projects: ['Site'],
	priorities: ['Urgente', 'Normale'],
	assignees: ['u1'],
	labels: ['infra'],
}

const EMPTY = {
	clients: [],
	projects: [],
	priorities: [],
	assignees: [],
	labels: [],
}

describe('KanbanToolbar', () => {
	it('filters by client with the existing chip and keeps the other choices', async () => {
		const onChange = vi.fn()
		render(
			<KanbanToolbar
				filters={{ ...NO_FILTERS, projects: ['Site'] }}
				options={OPTIONS}
				onChange={onChange}
			/>,
		)

		await userEvent.click(screen.getByRole('button', { name: 'Client' }))
		await userEvent.click(
			await screen.findByRole('menuitemcheckbox', { name: 'Acme' }),
		)

		expect(onChange).toHaveBeenCalledWith({
			...NO_FILTERS,
			clients: ['Acme'],
			projects: ['Site'],
		})
	})

	it('offers Priorité, Responsable and Étiquette next to Client and Projet', async () => {
		const onChange = vi.fn()
		render(
			<KanbanToolbar
				filters={NO_FILTERS}
				options={OPTIONS}
				onChange={onChange}
			/>,
		)
		const names = screen
			.getAllByRole('button')
			.map((button) => button.textContent)
		expect(names).toEqual([
			'Client',
			'Projet',
			'Priorité',
			'Responsable',
			'Étiquette',
		])

		await userEvent.click(screen.getByRole('button', { name: 'Priorité' }))
		await userEvent.click(
			await screen.findByRole('menuitemcheckbox', { name: 'Urgente' }),
		)
		expect(onChange).toHaveBeenLastCalledWith({
			...NO_FILTERS,
			priorities: ['Urgente'],
		})
	})

	it('chooses a responsable and a label', async () => {
		const onChange = vi.fn()
		render(
			<KanbanToolbar
				filters={{ ...NO_FILTERS, priorities: ['Haute'] }}
				options={OPTIONS}
				assigneeLabels={new Map([['u1', 'Claire Martin']])}
				onChange={onChange}
			/>,
		)

		await userEvent.click(screen.getByRole('button', { name: 'Responsable' }))
		await userEvent.click(
			await screen.findByRole('menuitemcheckbox', { name: 'Claire Martin' }),
		)
		expect(onChange).toHaveBeenLastCalledWith({
			...NO_FILTERS,
			priorities: ['Haute'],
			assignees: ['u1'],
		})
	})

	it('disables a chip with nothing to choose from', () => {
		render(
			<KanbanToolbar
				filters={NO_FILTERS}
				options={{ ...EMPTY }}
				onChange={vi.fn()}
			/>,
		)

		for (const name of ['Client', 'Priorité', 'Responsable', 'Étiquette']) {
			expect(
				(screen.getByRole('button', { name }) as HTMLButtonElement).disabled,
			).toBe(true)
		}
	})
})
