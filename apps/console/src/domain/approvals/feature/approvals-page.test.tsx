import { fireEvent, screen } from '@testing-library/react'
import { act } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { ApprovalsPage } from '#/domain/approvals/feature/approvals-page'
import { addApproval, demoStore, resetDemoState } from '#/domain/demo'

beforeEach(() => resetDemoState())
afterEach(() => vi.restoreAllMocks())

const EXTRA = ['/tasks', '/tasks/$taskId']

const pending = (id: string) => ({
	id,
	title: `Demande ${id}`,
	detail: 'Détail',
	requestedBy: 'dev',
	status: 'pending' as const,
	at: "à l'instant",
	decidedBy: '',
})

describe('ApprovalsPage', () => {
	it('renders the empty state, banner and history without network', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		await renderAtPath(<ApprovalsPage />, '/approvals', EXTRA)
		expect(
			await screen.findByRole('heading', { name: 'Approbations' }),
		).toBeTruthy()
		expect(screen.getByText(/démonstration/i)).toBeTruthy()
		expect(screen.getByText('Aucune approbation en attente')).toBeTruthy()
		expect(screen.getByText('Historique récent')).toBeTruthy()
		expect(screen.getAllByText('Approuvée')).toHaveLength(2)
		expect(screen.getAllByText('Refusée')).toHaveLength(1)
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('lists a pending approval with requester and approves it', async () => {
		act(() => addApproval(pending('APP-4')))
		await renderAtPath(<ApprovalsPage />, '/approvals', EXTRA)
		expect(await screen.findByText('Demande APP-4')).toBeTruthy()
		expect(screen.getByText(/Demandée par Dev/)).toBeTruthy()
		fireEvent.click(
			screen.getByRole('button', { name: 'Approuver : Demande APP-4' }),
		)
		expect(screen.getByText('Approbation accordée')).toBeTruthy()
		expect(screen.getByText('Aucune approbation en attente')).toBeTruthy()
		const decided = demoStore.state.approvals.find((a) => a.id === 'APP-4')
		expect(decided?.status).toBe('approved')
		expect(decided?.decidedBy).toBe('Board')
		expect(screen.getByText('Approuvée par Board')).toBeTruthy()
	})

	it('rejects a pending approval', async () => {
		act(() => addApproval(pending('APP-5')))
		await renderAtPath(<ApprovalsPage />, '/approvals', EXTRA)
		fireEvent.click(
			await screen.findByRole('button', { name: 'Refuser : Demande APP-5' }),
		)
		expect(screen.getByText('Approbation refusée')).toBeTruthy()
		expect(
			demoStore.state.approvals.find((a) => a.id === 'APP-5')?.status,
		).toBe('rejected')
	})

	it('links the task of an approval', async () => {
		act(() => addApproval({ ...pending('APP-6'), taskId: 'NAT-691' }))
		await renderAtPath(<ApprovalsPage />, '/approvals', EXTRA)
		const link = await screen.findByRole('link', { name: 'NAT-691' })
		expect(link.getAttribute('href')).toBe('/tasks/nat-691')
	})
})
