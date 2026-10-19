import { describe, expect, it } from 'vitest'
import {
	AGENT_CATALOG,
	agentInfo,
	agentModel,
	displayName,
} from '#/domain/agents/catalog'
import type { ModelAssignments } from '#/domain/settings/roles'

const ASSIGNMENTS: ModelAssignments = {
	extractor: { provider: 'anthropic', model: 'claude-sonnet' },
	judge: { provider: 'mistral', model: 'mistral-small' },
	writer: { provider: 'anthropic', model: 'claude-haiku' },
	embeddings: { provider: 'mistral', model: 'mistral-embed' },
	executor: 'claude-opus',
}

describe('agent catalog', () => {
	it('knows the four shipped agents', () => {
		for (const id of [
			'capitainerie-agent-extractor',
			'capitainerie-agent-judge',
			'capitainerie-agent-writer',
			'capitainerie-agent-executor',
		]) {
			const info = AGENT_CATALOG[id]
			expect(info?.name).toBeTruthy()
			expect(info?.description.endsWith('.')).toBe(true)
		}
		expect(AGENT_CATALOG['capitainerie-agent-executor']?.source).toBe(
			'Exécutant',
		)
		expect(AGENT_CATALOG['capitainerie-agent-writer']?.source).toBe('Pipeline')
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
})
