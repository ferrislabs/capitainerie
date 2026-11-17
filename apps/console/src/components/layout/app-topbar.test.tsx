import { screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { AppTopbar } from '#/components/layout/app-topbar'
import { renderAtPath } from '#/components/layout/test-router'

const CRUMBS = [
	{ label: 'Accueil', to: '/' },
	{ label: 'Runs', to: '/runs' },
	{ label: 'abc' },
]

async function renderBar(
	overrides: Partial<Parameters<typeof AppTopbar>[0]> = {},
) {
	const props = {
		crumbs: CRUMBS,
		onSearch: vi.fn(),
		dark: false,
		onToggleTheme: vi.fn(),
		...overrides,
	}
	await renderAtPath(<AppTopbar {...props} />, '/runs')
	return props
}

describe('AppTopbar', () => {
	it('links the parent crumbs and marks the last one current', async () => {
		await renderBar()
		expect(
			screen.getByRole('link', { name: 'Runs' }).getAttribute('href'),
		).toBe('/runs')
		expect(screen.getByText('abc').getAttribute('aria-current')).toBe('page')
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
})
