import { screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { TabNav } from '#/components/layout/tab-nav'
import { renderAtPath } from '#/components/layout/test-router'

const TABS = [
	{ label: 'Runs', link: { to: '/runs' } },
	{ label: 'Kanban', link: { to: '/kanban' } },
] as const

describe('TabNav', () => {
	it('renders one link per tab', async () => {
		await renderAtPath(<TabNav label="Sections" tabs={TABS} />, '/runs')
		expect(screen.getByRole('navigation', { name: 'Sections' })).toBeTruthy()
		expect(
			screen.getByRole('link', { name: 'Kanban' }).getAttribute('href'),
		).toBe('/kanban')
	})

	it('marks only the tab of the current URL', async () => {
		await renderAtPath(<TabNav label="Sections" tabs={TABS} />, '/kanban')
		expect(
			screen.getByRole('link', { name: 'Kanban' }).getAttribute('aria-current'),
		).toBe('page')
		expect(
			screen.getByRole('link', { name: 'Runs' }).getAttribute('aria-current'),
		).toBeNull()
	})
})
