import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { KanbanToolbar } from '#/domain/kanban/ui/kanban-toolbar'

describe('KanbanToolbar', () => {
	it('filters by client with the existing chip and keeps the project choice', async () => {
		const onChange = vi.fn()
		render(
			<KanbanToolbar
				filters={{ clients: [], projects: ['Site'] }}
				clients={['Acme', 'Globex']}
				projects={['Site']}
				onChange={onChange}
			/>,
		)

		await userEvent.click(screen.getByRole('button', { name: 'Client' }))
		await userEvent.click(
			await screen.findByRole('menuitemcheckbox', { name: 'Acme' }),
		)

		expect(onChange).toHaveBeenCalledWith({
			clients: ['Acme'],
			projects: ['Site'],
		})
	})

	it('disables a chip with nothing to choose from', () => {
		render(
			<KanbanToolbar
				filters={{ clients: [], projects: [] }}
				clients={[]}
				projects={[]}
				onChange={vi.fn()}
			/>,
		)

		expect(
			(screen.getByRole('button', { name: 'Client' }) as HTMLButtonElement)
				.disabled,
		).toBe(true)
	})
})
