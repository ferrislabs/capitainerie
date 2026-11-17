import { screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { TabNav } from '#/components/layout/tab-nav'
import { renderAtPath } from '#/components/layout/test-router'

const TABS = [
	{ label: 'Tâches', link: { to: '/tasks' } },
	{ label: 'Routines', link: { to: '/routines' } },
] as const

describe('TabNav', () => {
	it('renders one link per tab', async () => {
		await renderAtPath(<TabNav label="Sections" tabs={TABS} />, '/tasks')
		expect(screen.getByRole('navigation', { name: 'Sections' })).toBeTruthy()
		expect(
			screen.getByRole('link', { name: 'Routines' }).getAttribute('href'),
		).toBe('/routines')
	})

	it('marks only the tab of the current URL', async () => {
		await renderAtPath(<TabNav label="Sections" tabs={TABS} />, '/routines')
		expect(
			screen
				.getByRole('link', { name: 'Routines' })
				.getAttribute('aria-current'),
		).toBe('page')
		expect(
			screen.getByRole('link', { name: 'Tâches' }).getAttribute('aria-current'),
		).toBeNull()
	})
})
