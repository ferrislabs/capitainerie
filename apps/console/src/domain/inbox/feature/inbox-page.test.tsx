import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { InboxPage } from '#/domain/inbox/feature/inbox-page'
import type { InboxTab } from '#/domain/inbox/tabs'

async function open(tab: InboxTab) {
	await renderAtPath(
		<InboxPage tab={tab} />,
		tab === 'tous' ? '/inbox' : `/inbox/${tab}`,
		['/inbox/{-$tab}'],
	)
}

afterEach(() => vi.restoreAllMocks())

describe('InboxPage', () => {
	it('renders title, banner, tabs and all items without fetching', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		await open('tous')
		expect(
			await screen.findByRole('heading', { name: 'Boîte de réception' }),
		).toBeTruthy()
		expect(screen.getByText(/Données de démonstration/)).toBeTruthy()
		const nav = screen.getByRole('navigation', {
			name: 'Filtres de la boîte de réception',
		})
		expect(
			within(nav)
				.getByRole('link', { name: 'Tous' })
				.getAttribute('aria-current'),
		).toBe('page')
		expect(screen.getAllByRole('listitem')).toHaveLength(6)
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it('filters by the URL tab', async () => {
		await open('non-lus')
		await screen.findByRole('heading', { name: 'Boîte de réception' })
		expect(screen.getAllByRole('listitem')).toHaveLength(4)
	})

	it('shows an empty state once everything is marked read on non-lus', async () => {
		await open('non-lus')
		await userEvent.click(
			await screen.findByRole('button', { name: 'Tout marquer comme lu' }),
		)
		expect(screen.getByText('Rien ici')).toBeTruthy()
		expect(screen.queryAllByRole('listitem')).toHaveLength(0)
	})
})
