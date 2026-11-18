import { beforeEach, describe, expect, it } from 'vitest'
import { buildDefaultConfig, buildForm } from '#/domain/agent-team/agent-config'
import {
	getAgentConfig,
	resetAgentConfigs,
} from '#/domain/agent-team/agent-config-store'
import { agentPatch, recordRevision } from '#/domain/agent-team/save-agent'
import { DEV } from '#/test-utils/team-agents'

const base = () => buildForm(DEV, buildDefaultConfig(DEV))

describe('agentPatch', () => {
	it('is empty when nothing changed or only local fields changed', () => {
		expect(agentPatch(base(), base())).toEqual({})
		expect(agentPatch(base(), { ...base(), effort: 'Élevé' })).toEqual({})
	})

	it('maps the form to API fields', () => {
		expect(
			agentPatch(base(), {
				...base(),
				name: ' Dev 2 ',
				description: 'd',
				role: 'DevOps',
				model: 'claude-haiku-5-5',
				instr: 'nouvelles',
				reports: 'a-legal',
			}),
		).toEqual({
			name: 'Dev 2',
			description: 'd',
			role: 'DevOps',
			model: 'claude-haiku-5-5',
			instructions: 'nouvelles',
			reports_to: 'a-legal',
		})
	})

	it('keeps the current name when it is emptied', () => {
		expect(agentPatch(base(), { ...base(), name: '  ' })).toEqual({})
	})

	it('sends a null parent for the direction', () => {
		expect(
			agentPatch(base(), { ...base(), reports: 'Aucun (direction)' }),
		).toEqual({ reports_to: null })
	})
})

describe('recordRevision', () => {
	beforeEach(resetAgentConfigs)

	it('bumps the revision, lists changed fields and keeps local fields', () => {
		const next = { ...base(), model: 'claude-haiku-5-5', effort: 'Élevé' }
		expect(recordRevision(DEV, base(), next)).toBe(1)
		const config = getAgentConfig(DEV.id)
		expect(config.rev).toBe(1)
		expect(config.revs[0]?.changed).toBe('modèle, effort de réflexion')
		expect(config.fields.effort).toBe('Élevé')
		expect(recordRevision(DEV, next, { ...next, chrome: true })).toBe(2)
	})
})
