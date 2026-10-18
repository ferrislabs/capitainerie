import { fireEvent, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { AppRail } from '#/components/layout/app-rail'
import { NAV_SECTIONS } from '#/components/layout/nav-items'
import { renderAtPath } from '#/components/layout/test-router'
import { TooltipProvider } from '#/components/ui/tooltip'

function rail(
	overrides: Partial<Parameters<typeof AppRail>[0]> = {},
): Parameters<typeof AppRail>[0] {
	return {
		sections: NAV_SECTIONS,
		expanded: true,
		canExpand: true,
		onExpandedChange: vi.fn(),
		...overrides,
	}
}

async function renderRail(path: string, props = rail()) {
	await renderAtPath(
		<TooltipProvider>
			<AppRail {...props} />
		</TooltipProvider>,
		path,
	)
	return props
}

describe('AppRail', () => {
	it('groups the destinations by section and marks the current page', async () => {
		await renderRail('/kanban')

		expect(
			screen.getByRole('navigation', { name: 'Navigation principale' }),
		).toBeTruthy()
		for (const section of ['Travail', 'Supervision', 'Paramètres']) {
			expect(screen.getByRole('list', { name: section })).toBeTruthy()
		}
		for (const label of [
			'Runs',
			'Kanban',
			'Agents',
			'Fournisseurs',
			'Modèles',
		]) {
			expect(screen.getByRole('link', { name: label })).toBeTruthy()
		}
		expect(
			screen.getByRole('link', { name: 'Kanban' }).getAttribute('aria-current'),
		).toBe('page')
		expect(
			screen.getByRole('link', { name: 'Runs' }).getAttribute('aria-current'),
		).toBeNull()
	})

	it('marks a settings page as current', async () => {
		await renderRail('/settings/models')

		expect(
			screen
				.getByRole('link', { name: 'Modèles' })
				.getAttribute('aria-current'),
		).toBe('page')
		expect(
			screen
				.getByRole('link', { name: 'Fournisseurs' })
				.getAttribute('aria-current'),
		).toBeNull()
	})

	it('names icon-only links when collapsed', async () => {
		await renderRail('/runs', rail({ expanded: false }))

		expect(
			screen.getByRole('link', { name: 'Agents' }).getAttribute('aria-label'),
		).toBe('Agents')
		expect(screen.getByRole('button', { name: 'Déplier le menu' })).toBeTruthy()
	})

	it('toggles the expanded state', async () => {
		const props = await renderRail('/runs')

		await userEvent.click(
			screen.getByRole('button', { name: 'Réduire le menu' }),
		)

		expect(props.onExpandedChange).toHaveBeenCalledWith(false)
	})

	it('hides the toggle when the screen is too narrow', async () => {
		await renderRail('/runs', rail({ canExpand: false }))

		expect(screen.queryByRole('button', { name: /menu/ })).toBeNull()
	})

	it('renders the footer with the expanded state', async () => {
		await renderRail(
			'/runs',
			rail({
				expanded: false,
				footer: (expanded) => <p>{expanded ? 'large' : 'compact'}</p>,
			}),
		)

		expect(screen.getByText('compact')).toBeTruthy()
	})

	it('moves focus with the arrow keys across sections', async () => {
		await renderRail('/runs')
		const nav = screen.getByRole('navigation', {
			name: 'Navigation principale',
		})
		const runs = screen.getByRole('link', { name: 'Runs' })
		runs.focus()

		fireEvent.keyDown(nav, { key: 'ArrowDown' })
		expect(document.activeElement).toBe(
			screen.getByRole('link', { name: 'Kanban' }),
		)

		fireEvent.keyDown(nav, { key: 'ArrowDown' })
		expect(document.activeElement).toBe(
			screen.getByRole('link', { name: 'Agents' }),
		)

		fireEvent.keyDown(nav, { key: 'End' })
		expect(document.activeElement).toBe(
			screen.getByRole('link', { name: 'Modèles' }),
		)

		fireEvent.keyDown(nav, { key: 'ArrowDown' })
		expect(document.activeElement).toBe(runs)
	})
})
