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
	it('links the brand and the first entry to the home page', async () => {
		await renderRail('/')

		const brand = screen.getByRole('link', { name: 'Capitainerie, accueil' })
		expect(brand.getAttribute('href')).toBe('/')
		const home = screen.getByRole('link', { name: 'Accueil' })
		expect(home.getAttribute('href')).toBe('/')
		expect(home.getAttribute('aria-current')).toBe('page')
	})

	it('does not mark Accueil current on other pages', async () => {
		await renderRail('/runs')

		const home = screen.getByRole('link', { name: 'Accueil' })
		expect(home.getAttribute('aria-current')).toBeNull()
	})

	it('shows the number of active runs in the running blue', async () => {
		await renderRail('/live', rail({ badges: { '/live': 3 } }))

		const link = screen.getByRole('link', { name: 'En cours, 3 runs actifs' })
		const count = screen.getByText('3')
		expect(link.contains(count)).toBe(true)
		expect(count.className).toContain('text-signal-running')
		expect(count.className).not.toContain('attention')
	})

	it('caps the count at 100+', async () => {
		await renderRail('/live', rail({ badges: { '/live': 100 } }))

		expect(screen.getByText('100+')).toBeTruthy()
	})

	it('shows no count when nothing runs', async () => {
		await renderRail('/live', rail({ badges: { '/live': 0 } }))

		expect(screen.getByRole('link', { name: 'En cours' })).toBeTruthy()
		expect(screen.queryByText('0')).toBeNull()
	})

	it('groups the destinations by section and marks the current page', async () => {
		await renderRail('/kanban')

		expect(
			screen.getByRole('navigation', { name: 'Navigation principale' }),
		).toBeTruthy()
		for (const section of [
			'Travail',
			'Organisation',
			'Supervision',
			'Paramètres',
		]) {
			expect(screen.getByRole('list', { name: section })).toBeTruthy()
		}
		for (const label of [
			'Conversations',
			'Runs',
			'En cours',
			'Kanban',
			'Routines',
			'Agents',
			'Sources',
			'Connecteurs',
			'Audit',
			'Fournisseurs',
			'Modèles',
			'Intégrations',
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

	it('marks a conversation as current under Conversations', async () => {
		await renderRail('/chat/c1')

		expect(
			screen
				.getByRole('link', { name: 'Conversations' })
				.getAttribute('aria-current'),
		).toBe('page')
		expect(
			screen
				.getByRole('link', { name: 'Accueil' })
				.getAttribute('aria-current'),
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

	it('marks the integrations page as current', async () => {
		await renderRail('/settings/integrations')

		expect(
			screen
				.getByRole('link', { name: 'Intégrations' })
				.getAttribute('aria-current'),
		).toBe('page')
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
		screen.getByRole('link', { name: 'Runs' }).focus()

		for (const label of [
			'Kanban',
			'Tâches',
			'Clients',
			'Projets',
			'Routines',
			'Artifacts',
			'Agents',
			'Compétences',
			'Sources',
			'Connecteurs',
			'Approbations',
			'En cours',
			'Audit',
		]) {
			fireEvent.keyDown(nav, { key: 'ArrowDown' })
			expect(document.activeElement).toBe(
				screen.getByRole('link', { name: label }),
			)
		}

		fireEvent.keyDown(nav, { key: 'End' })
		expect(document.activeElement).toBe(
			screen.getByRole('link', { name: 'Intégrations' }),
		)

		fireEvent.keyDown(nav, { key: 'ArrowDown' })
		expect(document.activeElement).toBe(
			screen.getByRole('link', { name: 'Accueil' }),
		)
	})

	it('lists the live agents and the quick actions when expanded', async () => {
		const onNewTask = vi.fn()
		await renderRail(
			'/',
			rail({
				onNewTask,
				onSearch: vi.fn(),
				liveAgents: [{ id: 'dev', name: 'Dev' }],
			}),
		)

		const agent = screen.getByRole('link', { name: 'Dev' })
		expect(agent.getAttribute('href')).toBe('/agents/equipe/dev')
		await userEvent.click(
			screen.getByRole('button', { name: 'Nouvelle tâche' }),
		)
		expect(onNewTask).toHaveBeenCalledOnce()
	})

	it('hides the live agents when collapsed', async () => {
		await renderRail(
			'/',
			rail({ expanded: false, liveAgents: [{ id: 'dev', name: 'Dev' }] }),
		)

		expect(screen.queryByRole('link', { name: 'Dev' })).toBeNull()
	})
})
