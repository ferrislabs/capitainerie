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
	it('links the dashboard to the home page', async () => {
		await renderRail('/')

		const home = screen.getByRole('link', { name: /^Tableau de bord/ })
		expect(home.getAttribute('href')).toBe('/')
		expect(home.getAttribute('aria-current')).toBe('page')
	})

	it('does not mark the dashboard current on other pages', async () => {
		await renderRail('/tasks')

		expect(
			screen
				.getByRole('link', { name: /^Tableau de bord/ })
				.getAttribute('aria-current'),
		).toBeNull()
		expect(
			screen.getByRole('link', { name: 'Tâches' }).getAttribute('aria-current'),
		).toBe('page')
	})

	it('shows a count next to a destination', async () => {
		await renderRail('/', rail({ badges: { '/sources': 3 } }))

		const link = screen.getByRole('link', { name: /^Sources/ })
		expect(link.contains(screen.getByText('3'))).toBe(true)
	})

	it('caps the count at 100+', async () => {
		await renderRail('/', rail({ badges: { '/sources': 100 } }))

		expect(screen.getByText('100+')).toBeTruthy()
	})

	it('keeps a zero count visible, as in the mockup', async () => {
		await renderRail('/', rail({ badges: { '/approvals': 0 } }))

		expect(screen.getByText('0')).toBeTruthy()
	})

	it('labels the dashboard with the live count', async () => {
		await renderRail('/')

		expect(screen.getByText('1 en direct')).toBeTruthy()
	})

	it('opens the organization switcher', async () => {
		const onOrganization = vi.fn()
		await renderRail('/', rail({ onOrganization }))

		await userEvent.click(screen.getByRole('button', { name: 'Natalia' }))
		expect(onOrganization).toHaveBeenCalledOnce()
	})

	it('groups the destinations by section', async () => {
		await renderRail('/agents')

		expect(
			screen.getByRole('navigation', { name: 'Navigation principale' }),
		).toBeTruthy()
		for (const section of ['Général', 'Travail', 'Organisation']) {
			expect(screen.getByRole('list', { name: section })).toBeTruthy()
		}
		for (const label of [
			'Boîte de réception',
			'Tâches',
			'Projets',
			'Routines',
			'Artifacts',
			'Agents',
			'Compétences',
			'Sources',
			'Connecteurs',
			'Approbations',
			'Audit',
		]) {
			expect(screen.getByRole('link', { name: label })).toBeTruthy()
		}
		expect(
			screen.getByRole('link', { name: 'Agents' }).getAttribute('aria-current'),
		).toBe('page')
	})

	it('keeps the agent list marked under Agents', async () => {
		await renderRail('/agents/organigramme')

		expect(
			screen.getByRole('link', { name: 'Agents' }).getAttribute('aria-current'),
		).toBe('page')
	})

	it('collapses to icons with an accessible name', async () => {
		await renderRail('/', rail({ expanded: false }))

		expect(
			screen.getByRole('link', { name: 'Agents' }).getAttribute('aria-label'),
		).toBe('Agents')
		expect(screen.getByRole('button', { name: 'Déplier le menu' })).toBeTruthy()
	})

	it('toggles the expanded state', async () => {
		const props = await renderRail('/')

		await userEvent.click(
			screen.getByRole('button', { name: 'Réduire le menu' }),
		)

		expect(props.onExpandedChange).toHaveBeenCalledWith(false)
	})

	it('hides the toggle when the screen is too narrow', async () => {
		await renderRail('/', rail({ canExpand: false }))

		expect(screen.queryByRole('button', { name: /menu/ })).toBeNull()
	})

	it('renders the footer with the expanded state', async () => {
		await renderRail(
			'/',
			rail({
				expanded: false,
				footer: (expanded) => <p>{expanded ? 'large' : 'compact'}</p>,
			}),
		)

		expect(screen.getByText('compact')).toBeTruthy()
	})

	it('moves focus with the arrow keys across sections', async () => {
		await renderRail('/')
		const nav = screen.getByRole('navigation', {
			name: 'Navigation principale',
		})
		screen.getByRole('link', { name: /^Tableau de bord/ }).focus()

		for (const label of [
			'Boîte de réception',
			'Tâches',
			'Projets',
			'Routines',
			'Artifacts',
			'Agents',
			'Compétences',
			'Sources',
			'Connecteurs',
			'Approbations',
			'Audit',
		]) {
			fireEvent.keyDown(nav, { key: 'ArrowDown' })
			expect(document.activeElement).toBe(
				screen.getByRole('link', { name: label }),
			)
		}

		fireEvent.keyDown(nav, { key: 'ArrowDown' })
		expect(document.activeElement).toBe(
			screen.getByRole('link', { name: 'Voir tous les agents' }),
		)

		fireEvent.keyDown(nav, { key: 'ArrowDown' })
		expect(document.activeElement).toBe(
			screen.getByRole('link', { name: /^Tableau de bord/ }),
		)

		fireEvent.keyDown(nav, { key: 'End' })
		expect(document.activeElement).toBe(
			screen.getByRole('link', { name: 'Voir tous les agents' }),
		)
	})

	it('lists the live agents and the quick actions when expanded', async () => {
		const onNewTask = vi.fn()
		const onSearch = vi.fn()
		await renderRail(
			'/',
			rail({
				onNewTask,
				onSearch,
				liveAgents: [{ id: 'dev', name: 'Dev', hue: 120 }],
			}),
		)

		expect(
			screen.getByRole('link', { name: /^Dev/ }).getAttribute('href'),
		).toBe('/agents/dev')
		await userEvent.click(
			screen.getByRole('button', { name: 'Nouvelle tâche' }),
		)
		expect(onNewTask).toHaveBeenCalledOnce()
		await userEvent.click(screen.getByRole('button', { name: 'Rechercher' }))
		expect(onSearch).toHaveBeenCalledOnce()
		expect(screen.queryByText('⌘K')).toBeNull()
	})

	it('keeps the live agent avatars when collapsed, with an accessible name', async () => {
		await renderRail(
			'/',
			rail({
				expanded: false,
				liveAgents: [{ id: 'dev', name: 'Dev', hue: 120 }],
			}),
		)

		expect(screen.getByRole('link', { name: 'Dev' }).getAttribute('href')).toBe(
			'/agents/dev',
		)
		expect(screen.queryByText('Voir tous les agents')).toBeNull()
	})
})
