import { screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { AgentsTabs } from '#/domain/agent-team/ui/agents-tabs'

const EXTRA = ['/agents', '/agents/organigramme']

async function currentLabels() {
	return (await screen.findAllByRole('link'))
		.filter((l) => l.getAttribute('aria-current') === 'page')
		.map((l) => l.textContent)
}

describe('AgentsTabs', () => {
	it('lists the five mockup tabs', async () => {
		await renderAtPath(<AgentsTabs active="tous" />, '/agents', EXTRA)
		const links = await screen.findAllByRole('link')
		expect(links.map((l) => l.textContent)).toEqual([
			'Tous',
			'Actifs',
			'En pause',
			'Erreur',
			'Organigramme',
		])
	})

	it('marks only the active status tab and links it through the URL', async () => {
		await renderAtPath(<AgentsTabs active="actifs" />, '/agents', EXTRA)
		expect(await currentLabels()).toEqual(['Actifs'])
		expect(
			screen.getByRole('link', { name: 'En pause' }).getAttribute('href'),
		).toBe('/agents?filtre=pause')
	})

	it('marks the org chart tab', async () => {
		await renderAtPath(
			<AgentsTabs active="organigramme" />,
			'/agents/organigramme',
			EXTRA,
		)
		expect(await currentLabels()).toEqual(['Organigramme'])
	})
})
