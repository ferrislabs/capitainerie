import { screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { ConnectorsPage } from '#/domain/connectors/feature/connectors-page'
import { DEMO_CONNECTORS, resetDemoState } from '#/domain/demo'

const EXTRA = ['/connectors/{-$tab}', '/agents/equipe/$agentId/{-$tab}']

beforeEach(() => {
	resetDemoState()
	vi.restoreAllMocks()
})

describe('ConnectorsPage', () => {
	it('renders header, banner and installed connectors without network', async () => {
		const fetchSpy = vi.spyOn(globalThis, 'fetch')
		await renderAtPath(<ConnectorsPage tab={undefined} />, '/connectors', EXTRA)
		expect(
			screen.getByRole('heading', { name: 'Connecteurs', level: 1 }),
		).toBeTruthy()
		expect(screen.getByText(/démonstration/i)).toBeTruthy()
		const installed = DEMO_CONNECTORS.filter((c) => c.accounts.length > 0)
		expect(screen.getAllByRole('article')).toHaveLength(installed.length)
		expect(fetchSpy).not.toHaveBeenCalled()
	})

	it.each([
		['installes', 'Installés'],
		['canaux', 'Canaux'],
		['outils', 'Outils'],
		['registres', 'Registres'],
		['tous', 'Tous'],
	])('tab %s is current at its URL', async (slug, label) => {
		await renderAtPath(
			<ConnectorsPage tab={slug} />,
			`/connectors/${slug}`,
			EXTRA,
		)
		expect(
			screen.getByRole('link', { name: label }).getAttribute('aria-current'),
		).toBe('page')
	})

	it('tous lists every connector', async () => {
		await renderAtPath(<ConnectorsPage tab="tous" />, '/connectors/tous', EXTRA)
		expect(screen.getAllByRole('article')).toHaveLength(DEMO_CONNECTORS.length)
	})

	it('unknown tab falls back to installes', async () => {
		await renderAtPath(<ConnectorsPage tab="zzz" />, '/connectors', EXTRA)
		expect(
			screen
				.getByRole('link', { name: 'Installés' })
				.getAttribute('aria-current'),
		).toBe('page')
	})

	it('grants then revokes an agent', async () => {
		await renderAtPath(<ConnectorsPage tab={undefined} />, '/connectors', EXTRA)
		const first = DEMO_CONNECTORS.find((c) => c.accounts.length > 0)
		if (!first) throw new Error('fixture')
		const card = screen
			.getAllByRole('article')
			.find((a) =>
				within(a).queryByRole('heading', { name: new RegExp(first.name) }),
			)
		if (!card) throw new Error('card')
		const select = within(card).getByRole('combobox')
		const option = select.querySelector('option')
		const agentName = option?.textContent ?? ''
		await userEvent.click(
			within(card).getByRole('button', { name: /Autoriser/ }),
		)
		const short = agentName.split(' ')[0] ?? agentName
		const remove = within(card).getAllByRole('button', { name: /^Retirer / })
		expect(remove.length).toBeGreaterThan(0)
		const before = remove.length
		await userEvent.click(remove[0] as HTMLElement)
		expect(
			within(card).queryAllByRole('button', { name: /^Retirer / }),
		).toHaveLength(before - 1)
		expect(short).not.toBe('')
	})

	it('shows the denied message and no select without account', async () => {
		await renderAtPath(<ConnectorsPage tab="tous" />, '/connectors/tous', EXTRA)
		const bare = DEMO_CONNECTORS.find((c) => c.accounts.length === 0)
		if (!bare) return
		const card = screen
			.getAllByRole('article')
			.find((a) =>
				within(a).queryByRole('heading', { name: new RegExp(bare.name) }),
			)
		if (!card) throw new Error('card')
		expect(within(card).getByText('Aucun compte connecté')).toBeTruthy()
		expect(within(card).queryByRole('combobox')).toBeNull()
	})

	it('Configurer shows a notice', async () => {
		await renderAtPath(<ConnectorsPage tab={undefined} />, '/connectors', EXTRA)
		await userEvent.click(screen.getByRole('button', { name: 'Configurer' }))
		expect(screen.getByText('Instructions de connexion copiées')).toBeTruthy()
	})
})
