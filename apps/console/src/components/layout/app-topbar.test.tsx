import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { AppTopbar } from '#/components/layout/app-topbar'
import { renderAtPath } from '#/components/layout/test-router'

const CRUMBS = [
	{ label: 'Tâches', to: '/tasks' },
	{ label: 'Revue sécurité quotidienne', tag: 'NAT-611' },
]

async function renderBar(
	overrides: Partial<Parameters<typeof AppTopbar>[0]> = {},
) {
	const props = {
		crumbs: CRUMBS,
		organization: 'Acme',
		onSearch: vi.fn(),
		dark: false,
		onToggleTheme: vi.fn(),
		...overrides,
	}
	await renderAtPath(<AppTopbar {...props} />, '/tasks')
	return props
}

describe('AppTopbar', () => {
	it('starts with the organization, links the parent crumbs and marks the last current', async () => {
		await renderBar()
		expect(screen.getByText('Acme')).toBeTruthy()
		expect(
			screen.getByRole('link', { name: 'Tâches' }).getAttribute('href'),
		).toBe('/tasks')
		const last = screen.getByText('Revue sécurité quotidienne')
		expect(last.getAttribute('aria-current')).toBe('page')
		expect(screen.getByText('NAT-611')).toBeTruthy()
	})

	it('shows the real live count and no demo chip', async () => {
		await renderBar({ liveCount: 3 })
		expect(screen.queryByText('Démo')).toBeNull()
		expect(screen.getByText('3 en direct')).toBeTruthy()
	})

	it('hides the live label when nothing runs', async () => {
		await renderBar()
		expect(screen.queryByText(/en direct/)).toBeNull()
	})

	it('opens the search from the search field', async () => {
		const { onSearch } = await renderBar()
		await userEvent.click(screen.getByRole('button', { name: 'Rechercher' }))
		expect(onSearch).toHaveBeenCalledOnce()
	})

	it('offers the other theme', async () => {
		const { onToggleTheme } = await renderBar({ dark: true })
		await userEvent.click(
			screen.getByRole('button', { name: 'Passer au thème clair' }),
		)
		expect(onToggleTheme).toHaveBeenCalledOnce()
	})

	it('exposes the portal target for page actions', async () => {
		await renderBar()
		expect(document.getElementById('topbar-actions')).toBeTruthy()
	})

	it('refreshes the page data', async () => {
		const onRefresh = vi.fn()
		await renderBar({ onRefresh })
		await userEvent.click(screen.getByRole('button', { name: 'Actualiser' }))
		expect(onRefresh).toHaveBeenCalledOnce()
	})

	it('hides the refresh button on pages that bring their own toggle', async () => {
		await renderBar({ showRefresh: false })
		expect(screen.queryByRole('button', { name: 'Actualiser' })).toBeNull()
	})
})
