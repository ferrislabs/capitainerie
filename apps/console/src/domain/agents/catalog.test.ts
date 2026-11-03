import { describe, expect, it } from 'vitest'
import {
	AGENT_CATALOG,
	agentInfo,
	agentModel,
	agentModelChoice,
	agentRole,
	displayName,
} from '#/domain/agents/catalog'
import type { ModelAssignments } from '#/domain/settings/roles'

const ASSIGNMENTS: ModelAssignments = {
	extractor: { provider: 'anthropic', model: 'claude-sonnet' },
	judge: { provider: 'mistral', model: 'mistral-small' },
	writer: { provider: 'anthropic', model: 'claude-haiku' },
	embeddings: { provider: 'mistral', model: 'mistral-embed' },
	integrator: { provider: 'anthropic', model: 'claude-sonnet-5' },
	executor: 'claude-opus',
}

describe('agent catalog', () => {
	it('knows the five shipped agents', () => {
		for (const id of [
			'capitainerie-agent-extractor',
			'capitainerie-agent-judge',
			'capitainerie-agent-writer',
			'capitainerie-agent-executor',
			'capitainerie-agent-integrator',
		]) {
			const info = AGENT_CATALOG[id]
			expect(info?.name).toBeTruthy()
			expect(info?.description.endsWith('.')).toBe(true)
		}
		expect(AGENT_CATALOG['capitainerie-agent-executor']?.source).toBe(
			'Exécutant',
		)
		expect(AGENT_CATALOG['capitainerie-agent-writer']?.source).toBe('Pipeline')
		expect(AGENT_CATALOG['capitainerie-agent-integrator']?.name).toBe(
			'Intervenant',
		)
		expect(AGENT_CATALOG['capitainerie-agent-integrator']?.source).toBe(
			'Intervenant',
		)
	})

	it('falls back to the identifier for an unknown agent', () => {
		expect(agentInfo('capitainerie-agent-mystery')).toBeUndefined()
		expect(displayName('capitainerie-agent-mystery')).toBe(
			'capitainerie-agent-mystery',
		)
		expect(displayName('capitainerie-agent-writer')).toBe(
			AGENT_CATALOG['capitainerie-agent-writer']?.name,
		)
	})

	it('looks the model up through the role', () => {
		expect(agentModel('capitainerie-agent-writer', ASSIGNMENTS)).toBe(
			'claude-haiku',
		)
		expect(agentModel('capitainerie-agent-executor', ASSIGNMENTS)).toBe(
			'claude-opus',
		)
		expect(agentModel('capitainerie-agent-integrator', ASSIGNMENTS)).toBe(
			'claude-sonnet-5',
		)
	})

	it('has no model for an unknown agent or missing assignments', () => {
		expect(agentModel('other', ASSIGNMENTS)).toBeUndefined()
		expect(agentModel('capitainerie-agent-writer', undefined)).toBeUndefined()
		expect(
			agentModel('capitainerie-agent-executor', {
				...ASSIGNMENTS,
				executor: null,
			}),
		).toBeUndefined()
	})

	it('resolves the Captain, who is not in the catalog, through its role', () => {
		expect(agentInfo('capitainerie-agent-captain')).toBeUndefined()
		expect(agentRole('capitainerie-agent-captain')).toBe('captain')
		expect(agentRole('capitainerie-agent-integrator')).toBe('integrator')
		expect(agentRole('other')).toBeUndefined()
		expect(
			agentModelChoice('capitainerie-agent-captain', {
				...ASSIGNMENTS,
				captain: { provider: 'mistral', model: 'mistral-large' },
			}),
		).toEqual({ model: 'mistral-large', fallback: false })
	})

	it('falls back to the writer model for the Captain and the Intervenant', () => {
		const unset = { ...ASSIGNMENTS, captain: null, integrator: null }

		for (const id of [
			'capitainerie-agent-captain',
			'capitainerie-agent-integrator',
		]) {
			expect(agentModelChoice(id, unset)).toEqual({
				model: 'claude-haiku',
				fallback: true,
			})
			expect(agentModel(id, unset)).toBe('claude-haiku')
		}
		expect(
			agentModelChoice('capitainerie-agent-integrator', {
				...ASSIGNMENTS,
				integrator: undefined,
			}),
		).toEqual({ model: 'claude-haiku', fallback: true })
	})

	it('does not fall back for the other roles', () => {
		expect(
			agentModelChoice('capitainerie-agent-executor', {
				...ASSIGNMENTS,
				executor: null,
			}),
		).toBeUndefined()
		expect(agentModelChoice('capitainerie-agent-writer', ASSIGNMENTS)).toEqual({
			model: 'claude-haiku',
			fallback: false,
		})
	})
})
