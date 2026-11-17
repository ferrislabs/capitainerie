import { screen, within } from '@testing-library/react'
import { act } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { DashboardPage } from '#/domain/dashboard/feature/dashboard-page'
import { addApproval, demoStore, resetDemoState } from '#/domain/demo'

const EXTRA = [
	'/agents/$agentId/{-$tab}',
	'/audit/{-$tab}',
	'/tasks',
	'/tasks/$taskId',
]

beforeEach(() => resetDemoState())
afterEach(() => vi.restoreAllMocks())

describe('DashboardPage', () => {
	it('renders title, banner, tiles and panels without network', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		await renderAtPath(<DashboardPage />, '/', EXTRA)
		expect(
			await screen.findByRole('heading', { name: 'Tableau de bord' }),
		).toBeTruthy()
		expect(screen.getByText(/démonstration/i)).toBeTruthy()
		for (const label of [
			'Agents en direct',
			'Tâches ouvertes',
			'Tâches bloquées',
			'Approbations en attente',
		]) {
			expect(screen.getAllByText(label).length).toBeGreaterThan(0)
		}
		expect(screen.getByText('Exécutions par jour')).toBeTruthy()
		expect(screen.getByText('Activité récente')).toBeTruthy()
		expect(screen.getByText('Coûts du mois')).toBeTruthy()
		const chart = screen.getByRole('img')
		expect(chart.getAttribute('aria-label')).toContain('Exécutions par jour')
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('links to the pages and to the costs tab', async () => {
		await renderAtPath(<DashboardPage />, '/', EXTRA)
		await screen.findByRole('heading', { name: 'Tableau de bord' })
		const hrefs = screen.getAllByRole('link').map((l) => l.getAttribute('href'))
		expect(hrefs).toContain('/agents')
		expect(hrefs).toContain('/approvals')
		expect(hrefs).toContain('/audit/couts')
		expect(hrefs.some((h) => h?.startsWith('/agents/security-officer'))).toBe(
			true,
		)
	})

	it('links the task tiles and the live agents current task', async () => {
		await renderAtPath(<DashboardPage />, '/', EXTRA)
		await screen.findByRole('heading', { name: 'Tableau de bord' })
		for (const label of ['Tâches ouvertes', 'Tâches bloquées']) {
			const a = screen.getByText(label).closest('a')
			expect(a?.getAttribute('href')?.startsWith('/tasks')).toBe(true)
		}
		const task = demoStore.state.tasks.find(
			(t) => t.agentId === 'security-officer' && t.status !== 'done',
		)
		const link = screen.getByRole('link', { name: task?.id ?? '' })
		expect(link.getAttribute('href')).toBe(`/tasks/${task?.id.toLowerCase()}`)
	})

	it('counts pending approvals from the store', async () => {
		act(() =>
			addApproval({
				id: 'APP-4',
				title: 'x',
				detail: 'y',
				requestedBy: null,
				status: 'pending',
				at: "à l'instant",
				decidedBy: '',
			}),
		)
		await renderAtPath(<DashboardPage />, '/', EXTRA)
		const link = (await screen.findByText('Approbations en attente')).closest(
			'a',
		)
		expect(link).toBeTruthy()
		expect(within(link as HTMLElement).getByText('1')).toBeTruthy()
	})
})
