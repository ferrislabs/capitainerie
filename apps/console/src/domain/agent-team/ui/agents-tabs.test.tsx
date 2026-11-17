import { screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { renderAtPath } from '#/components/layout/test-router'
import { AgentsTabs } from '#/domain/agent-team/ui/agents-tabs'

const EXTRA = ['/agents/equipe', '/agents/organigramme']

describe('AgentsTabs', () => {
	it('marks only the costs tab on /agents', async () => {
		await renderAtPath(<AgentsTabs />, '/agents', EXTRA)
		const current = (await screen.findAllByRole('link')).filter(
			(l) => l.getAttribute('aria-current') === 'page',
		)
		expect(current.map((l) => l.textContent)).toEqual(['Activité et coûts'])
	})

	it('marks the org chart tab', async () => {
		await renderAtPath(<AgentsTabs />, '/agents/organigramme', EXTRA)
		expect(
			(await screen.findByRole('link', { name: 'Organigramme' })).getAttribute(
				'aria-current',
			),
		).toBe('page')
	})
})
