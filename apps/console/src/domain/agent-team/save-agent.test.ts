import { describe, expect, it } from 'vitest'
import { buildForm } from '#/domain/agent-team/agent-config'
import { agentPatch, changeSummary } from '#/domain/agent-team/save-agent'
import { DEV } from '#/test-utils/team-agents'

const base = () => buildForm(DEV)

describe('agentPatch', () => {
	it('is empty when nothing changed', () => {
		expect(agentPatch(DEV, base(), base())).toEqual({})
	})

	it('sends the whole config with a change summary when a config field changed', () => {
		const agent = { ...DEV, config: { legacy: 1 } }
		const patch = agentPatch(agent, base(), { ...base(), effort: 'Élevé' })
		expect(patch.change_summary).toBe('effort de réflexion')
		expect(patch.config).toMatchObject({ legacy: 1, effort: 'Élevé' })
		expect(patch.config).not.toHaveProperty('name')
		expect(patch).not.toHaveProperty('name')
	})

	it('maps the form to API fields', () => {
		expect(
			agentPatch(DEV, base(), {
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
			change_summary:
				'nom, description, rôle, hiérarchie, modèle, instructions',
		})
	})

	it('keeps the current name when it is emptied', () => {
		expect(agentPatch(DEV, base(), { ...base(), name: '  ' })).toEqual({})
	})

	it('sends a null parent for the direction', () => {
		expect(
			agentPatch(DEV, base(), { ...base(), reports: 'Aucun (direction)' }),
		).toEqual({ reports_to: null, change_summary: 'hiérarchie' })
	})

	it('caps the change summary at 2000 characters', () => {
		expect(changeSummary('x'.repeat(2500))).toHaveLength(2000)
		expect(changeSummary('court')).toBe('court')
	})
})
