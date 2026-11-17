import { screen, within } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { AuditPage } from '#/domain/audit/feature/audit-page'
import { AUDIT_TABS, type AuditTab } from '#/domain/audit/tabs'

const EXTRA = ['/audit/{-$tab}']

async function open(tab: AuditTab) {
	await renderAtPath(<AuditPage tab={tab} />, `/audit/${tab}`, EXTRA)
}

afterEach(() => vi.restoreAllMocks())

describe('AuditPage', () => {
	it('renders the title, the banner and five tab links', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		await open('activite')
		expect(await screen.findByRole('heading', { name: 'Audit' })).toBeTruthy()
		expect(screen.getByText(/Données de démonstration/)).toBeTruthy()
		const nav = screen.getByRole('navigation', { name: "Sections de l'audit" })
		expect(within(nav).getAllByRole('link')).toHaveLength(AUDIT_TABS.length)
		expect(
			within(nav)
				.getByRole('link', { name: 'Activité' })
				.getAttribute('aria-current'),
		).toBe('page')
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('lists the activity events', async () => {
		await open('activite')
		expect(
			await screen.findAllByText('Iris (Relais retours)'),
		).not.toHaveLength(0)
	})

	it('renders the runs table', async () => {
		await open('runs')
		expect(
			await screen.findByRole('columnheader', { name: 'Déclencheur' }),
		).toBeTruthy()
		expect(screen.getByText('en cours')).toBeTruthy()
		expect(screen.getByText('échec')).toBeTruthy()
	})

	it('renders tiles and the by-agent table', async () => {
		await open('couts')
		expect(await screen.findByText('Dépense API')).toBeTruthy()
		expect(screen.getByText('Événements financiers')).toBeTruthy()
		expect(
			screen.getByRole('columnheader', { name: 'Dont cache' }),
		).toBeTruthy()
	})

	it('renders one budget row per scope', async () => {
		await open('budgets')
		expect(await screen.findByText('0,00 $ sur 400 $ ce mois')).toBeTruthy()
		expect(screen.getAllByRole('progressbar')).toHaveLength(4)
	})

	it('renders the timeline with severity markers', async () => {
		await open('chronologie')
		expect(await screen.findByText('07:00')).toBeTruthy()
		expect(screen.getByRole('img', { name: 'Critique' })).toBeTruthy()
	})
})
