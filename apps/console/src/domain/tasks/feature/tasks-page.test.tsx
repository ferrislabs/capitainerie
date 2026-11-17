import { fireEvent, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { demoStore, resetDemoState } from '#/domain/demo'
import { TasksPage } from '#/domain/tasks/feature/tasks-page'

const EXTRA = ['/tasks', '/tasks/$taskId', '/agents/equipe/$agentId/{-$tab}']

describe('TasksPage', () => {
	beforeEach(resetDemoState)

	it('renders title, banner, toolbar and grouped list', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		await renderAtPath(<TasksPage view="liste" query="" />, '/tasks', EXTRA)
		expect(await screen.findByRole('heading', { name: 'Tâches' })).toBeTruthy()
		expect(screen.getAllByRole('status')[0]?.textContent).toContain(
			'démonstration',
		)
		expect(screen.getByRole('button', { name: 'Nouvelle tâche' })).toBeTruthy()
		expect(
			screen.getByRole('textbox', { name: 'Rechercher des tâches' }),
		).toBeTruthy()
		expect(screen.getByRole('region', { name: 'Bloqué' })).toBeTruthy()
		const link = screen
			.getAllByRole('link')
			.find((l) => l.textContent?.includes('NAT-695'))
		expect(link?.getAttribute('href')).toBe('/tasks/nat-695')
		expect(
			screen.getByRole('link', { name: 'Liste' }).getAttribute('aria-current'),
		).toBe('page')
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('renders the board with the six first statuses', async () => {
		await renderAtPath(<TasksPage view="tableau" query="" />, '/tasks', EXTRA)
		await screen.findByRole('heading', { name: 'Tâches' })
		expect(screen.getByRole('region', { name: 'Backlog' })).toBeTruthy()
		expect(screen.getByRole('region', { name: 'Terminé' })).toBeTruthy()
		expect(screen.queryByRole('region', { name: 'Annulé' })).toBeNull()
		expect(
			screen
				.getByRole('link', { name: 'Tableau' })
				.getAttribute('aria-current'),
		).toBe('page')
	})

	it('filters with the search field', async () => {
		await renderAtPath(<TasksPage view="liste" query="" />, '/tasks', EXTRA)
		const field = await screen.findByRole('textbox', {
			name: 'Rechercher des tâches',
		})
		fireEvent.change(field, { target: { value: 'NAT-695' } })
		expect(
			screen
				.getAllByRole('link')
				.filter((l) => l.textContent?.includes('NAT-6')).length,
		).toBe(1)
	})

	it('creates a task from the dialog and navigates to it', async () => {
		await renderAtPath(<TasksPage view="liste" query="" />, '/tasks', EXTRA)
		fireEvent.click(
			await screen.findByRole('button', { name: 'Nouvelle tâche' }),
		)
		fireEvent.click(
			await screen.findByRole('button', { name: 'Créer et assigner' }),
		)
		expect(await screen.findByText('Écris le titre de la tâche')).toBeTruthy()
		fireEvent.change(screen.getByLabelText('Titre'), {
			target: { value: 'Ma tâche' },
		})
		fireEvent.click(screen.getByRole('button', { name: 'Créer et assigner' }))
		const created = demoStore.state.tasks.find((t) => t.title === 'Ma tâche')
		expect(created?.status).toBe('todo')
		expect(created?.id).toBe('NAT-696')
	})

	it('closes the dialog on cancel without creating', async () => {
		const before = demoStore.state.tasks.length
		await renderAtPath(<TasksPage view="liste" query="" />, '/tasks', EXTRA)
		fireEvent.click(
			await screen.findByRole('button', { name: 'Nouvelle tâche' }),
		)
		fireEvent.click(await screen.findByRole('button', { name: 'Annuler' }))
		expect(
			screen.queryByRole('button', { name: 'Créer et assigner' }),
		).toBeNull()
		expect(demoStore.state.tasks.length).toBe(before)
	})
})
