import { fireEvent, screen } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { demoStore, resetDemoState, updateSource } from '#/domain/demo'
import { SourcesPage } from '#/domain/sources/feature/sources-page'

const EXTRA = ['/sources/nouvelle', '/sources/$sourceId/{-$tab}']

describe('SourcesPage', () => {
	beforeEach(resetDemoState)

	it('renders title, banner, table and new source button', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		await renderAtPath(<SourcesPage />, '/sources', EXTRA)
		expect(await screen.findByRole('heading', { name: 'Sources' })).toBeTruthy()
		expect(screen.getByRole('status').textContent).toContain('démonstration')
		expect(screen.getByRole('link', { name: 'Nouvelle source' })).toBeTruthy()
		const link = screen.getByRole('link', {
			name: 'Nouveaux sous-domaines sensibles',
		})
		expect(link.getAttribute('href')).toBe(
			'/sources/nouveaux-sous-domaines-sensibles',
		)
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('toggles a source in the store', async () => {
		await renderAtPath(<SourcesPage />, '/sources', EXTRA)
		const sw = await screen.findByRole('switch', {
			name: 'Activer Boîte du cabinet comptable',
		})
		fireEvent.click(sw)
		expect(
			demoStore.state.sources.find((s) => s.id === 'boite-du-cabinet-comptable')
				?.enabled,
		).toBe(false)
	})

	it('warns when a source has no agent', async () => {
		updateSource('pieces-recues-sur-chatwoot', { agentIds: [] })
		await renderAtPath(<SourcesPage />, '/sources', EXTRA)
		expect(await screen.findByText('Aucun agent')).toBeTruthy()
	})
})
