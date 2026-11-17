import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { act } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { DashboardPage } from '#/domain/dashboard/feature/dashboard-page'
import { addApproval, resetDemoState } from '#/domain/demo'

const EXTRA = [
	'/agents/$agentId/{-$tab}',
	'/audit/{-$tab}',
	'/tasks',
	'/tasks/$taskId',
]

beforeEach(() => resetDemoState())
afterEach(() => vi.restoreAllMocks())

async function renderDashboard() {
	await renderAtPath(<DashboardPage />, '/', EXTRA)
	await screen.findByRole('heading', { name: 'Tableau de bord' })
}

describe('DashboardPage', () => {
	it('renders the header, tiles, charts and panels without network', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		await renderDashboard()
		expect(screen.getByText(/démonstration/i)).toBeTruthy()
		expect(
			screen.getByText('Natalia · 27 agents · actualisé il y a 12 s'),
		).toBeTruthy()
		for (const label of [
			'Agents activés',
			'Tâches en cours',
			'Bloquées',
			'Dépense du mois',
			'Approbations',
		]) {
			expect(screen.getAllByText(label).length).toBeGreaterThan(0)
		}
		expect(screen.getByText('122 ouvertes')).toBeTruthy()
		expect(screen.getByText('+8 depuis hier')).toBeTruthy()
		expect(screen.getByText('0 % du budget de 400 $')).toBeTruthy()
		for (const title of [
			'Activité des exécutions',
			'Tâches par statut',
			'Taux de réussite',
		]) {
			expect(
				screen.getByRole('img', { name: `${title}, 14 derniers jours` }),
			).toBeTruthy()
		}
		for (const title of ['À traiter', 'Activité récente', 'Tâches récentes']) {
			expect(screen.getByRole('heading', { name: title })).toBeTruthy()
		}
		expect(screen.getAllByText('Critique')).toHaveLength(2)
		expect(screen.getAllByText('Attention')).toHaveLength(2)
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('shows the live run cards and the other runs link', async () => {
		await renderDashboard()
		expect(
			screen.getByRole('heading', { name: 'Agents en direct' }),
		).toBeTruthy()
		expect(screen.getByText('Security Officer (RSSI + Blue Team)')).toBeTruthy()
		expect(screen.getAllByText('Heartbeat planifié')).toHaveLength(3)
		const other = screen.getByRole('link', {
			name: '46 autres exécutions actives ou récentes',
		})
		expect(other.getAttribute('href')).toBe('/audit')
	})

	it('links to the pages and to the costs tab', async () => {
		await renderDashboard()
		const hrefs = screen.getAllByRole('link').map((l) => l.getAttribute('href'))
		expect(hrefs).toContain('/agents')
		expect(hrefs).toContain('/approvals')
		expect(hrefs).toContain('/audit/couts')
		expect(hrefs).toContain('/audit/runs')
		expect(hrefs).toContain('/tasks/nat-611')
		expect(
			screen
				.getByRole('link', { name: "Journal d'audit" })
				.getAttribute('href'),
		).toBe('/audit')
	})

	it('links the recent tasks to their page', async () => {
		await renderDashboard()
		const link = screen.getByRole('link', { name: /NAT-695/ })
		expect(link.getAttribute('href')).toBe('/tasks/nat-695')
		expect(within(link).getByText('il y a 20 min')).toBeTruthy()
	})

	it('links the task tiles to the task list', async () => {
		await renderDashboard()
		for (const label of ['Tâches en cours', 'Bloquées']) {
			const a = screen.getByText(label).closest('a')
			expect(a?.getAttribute('href')?.startsWith('/tasks')).toBe(true)
		}
	})

	it('opens the new task dialog', async () => {
		await renderDashboard()
		await userEvent.click(
			screen.getByRole('button', { name: 'Nouvelle tâche' }),
		)
		expect(await screen.findByRole('dialog')).toBeTruthy()
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
		const link = (await screen.findByText('En attente du board')).closest('a')
		expect(link).toBeTruthy()
		expect(within(link as HTMLElement).getByText('1')).toBeTruthy()
	})
})
