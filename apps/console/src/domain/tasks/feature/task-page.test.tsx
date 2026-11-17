import { fireEvent, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { addTask, demoStore, resetDemoState } from '#/domain/demo'
import { TaskPage } from '#/domain/tasks/feature/task-page'
import { resetComments } from '#/domain/tasks/mock/comments-store'

const EXTRA = [
	'/tasks',
	'/tasks/$taskId',
	'/agents/equipe/$agentId/{-$tab}',
	'/sources/$sourceId/{-$tab}',
]

const open = (pathId: string) =>
	renderAtPath(<TaskPage pathId={pathId} />, `/tasks/${pathId}`, EXTRA)

describe('TaskPage', () => {
	beforeEach(() => {
		resetDemoState()
		resetComments()
	})

	it('renders id, title, banners, dossier-less thread and no fetch', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		await open('nat-691')
		expect(await screen.findByRole('heading', { level: 1 })).toBeTruthy()
		expect(screen.getAllByRole('status')[0]?.textContent).toContain(
			'démonstration',
		)
		expect(screen.getByText('Déclenchée par une source')).toBeTruthy()
		expect(screen.getByText('auth.lumen-sante.com')).toBeTruthy()
		expect(screen.getByText(/Prise en charge/)).toBeTruthy()
		expect(screen.getByRole('link', { name: 'Retour aux tâches' })).toBeTruthy()
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('accepts uppercase ids and shows the parent banner', async () => {
		await open('NAT-694')
		expect(await screen.findByText(/Déléguée par/)).toBeTruthy()
		expect(screen.getByText(/Tâche traitée/)).toBeTruthy()
	})

	it('shows not found for an unknown id', async () => {
		await open('nat-9999')
		expect(await screen.findByText('Tâche introuvable')).toBeTruthy()
		expect(screen.getByRole('link', { name: 'Retour aux tâches' })).toBeTruthy()
	})

	it('shows sub-tasks progress', async () => {
		await open('nat-691')
		expect(await screen.findByText('1 / 2 terminées')).toBeTruthy()
	})

	it('updates the status in the store and hides the properties', async () => {
		await open('nat-691')
		fireEvent.change(await screen.findByRole('combobox', { name: 'Statut' }), {
			target: { value: 'review' },
		})
		expect(demoStore.state.tasks.find((t) => t.id === 'NAT-691')?.status).toBe(
			'review',
		)
		fireEvent.click(
			screen.getByRole('button', { name: 'Masquer les propriétés' }),
		)
		expect(screen.queryByRole('combobox', { name: 'Statut' })).toBeNull()
		fireEvent.click(
			screen.getByRole('button', { name: 'Afficher les propriétés' }),
		)
		expect(screen.getByRole('combobox', { name: 'Statut' })).toBeTruthy()
	})

	it('posts a comment locally', async () => {
		await open('nat-691')
		fireEvent.change(await screen.findByRole('textbox', { name: 'Message' }), {
			target: { value: 'Bonjour' },
		})
		fireEvent.click(screen.getByRole('button', { name: 'Envoyer' }))
		expect(await screen.findByText('Bonjour')).toBeTruthy()
	})

	it('delegates a sub-task then refuses past the parallel limit', async () => {
		await open('nat-691')
		const title = await screen.findByRole('textbox', {
			name: 'Titre de la sous-tâche',
		})
		fireEvent.change(title, { target: { value: 'Vérifier le SIREN' } })
		fireEvent.click(screen.getByRole('button', { name: 'Déléguer' }))
		const created = demoStore.state.tasks.find(
			(t) => t.title === 'Vérifier le SIREN',
		)
		expect(created?.parentId).toBe('NAT-691')
		expect(created?.status).toBe('todo')
		expect(created?.project).toBe('Veille sous-domaines')
		addTask({
			...(created as NonNullable<typeof created>),
			id: 'NAT-900',
			title: 'Autre',
		})
		fireEvent.change(title, { target: { value: 'Encore' } })
		fireEvent.click(screen.getByRole('button', { name: 'Déléguer' }))
		expect(
			await screen.findByText('Limite atteinte : 3 sous-tâches en parallèle'),
		).toBeTruthy()
	})

	it('refuses an empty delegation title', async () => {
		await open('nat-691')
		fireEvent.click(await screen.findByRole('button', { name: 'Déléguer' }))
		expect(
			await screen.findByText('Écris le titre de la sous-tâche'),
		).toBeTruthy()
	})
})
