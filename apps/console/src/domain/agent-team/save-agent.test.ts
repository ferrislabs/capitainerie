import { beforeEach, describe, expect, it } from 'vitest'
import {
	getAgentConfig,
	resetAgentConfigs,
} from '#/domain/agent-team/agent-config-store'
import { readAgentForm, saveAgentForm } from '#/domain/agent-team/save-agent'
import { demoStore, resetDemoState } from '#/domain/demo'

describe('saveAgentForm', () => {
	beforeEach(() => {
		resetDemoState()
		resetAgentConfigs()
	})

	it('does nothing when nothing changed', () => {
		const before = getAgentConfig('dev').rev
		expect(saveAgentForm('dev', readAgentForm('dev'))).toEqual([])
		expect(getAgentConfig('dev').rev).toBe(before)
	})

	it('bumps the revision and lists changed fields', () => {
		const before = getAgentConfig('dev').rev
		const form = {
			...readAgentForm('dev'),
			model: 'Claude Haiku 5.5',
			effort: 'Élevé',
		}
		expect(saveAgentForm('dev', form)).toEqual(['model', 'effort'])
		const config = getAgentConfig('dev')
		expect(config.rev).toBe(before + 1)
		expect(config.revs[0]?.changed).toBe('modèle, effort de réflexion')
		expect(config.fields.effort).toBe('Élevé')
		expect(demoStore.state.agents.find((a) => a.id === 'dev')?.model).toBe(
			'Claude Haiku 5.5',
		)
	})

	it('moves the hierarchy and renames without orphaning sub-agents', () => {
		const form = { ...readAgentForm('veille-legale'), name: 'Veille juridique' }
		saveAgentForm('veille-legale', form)
		expect(demoStore.state.reportsTo.registre).toBe('Veille juridique')
		saveAgentForm('dev', {
			...readAgentForm('dev'),
			reports: 'Aucun (direction)',
		})
		expect(demoStore.state.reportsTo.dev).toBeNull()
	})
})
