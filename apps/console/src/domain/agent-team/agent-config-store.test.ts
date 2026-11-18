import { beforeEach, describe, expect, it } from 'vitest'
import {
	getAgentConfig,
	resetAgentConfigs,
	updateAgentConfig,
} from '#/domain/agent-team/agent-config-store'

describe('agent config store', () => {
	beforeEach(resetAgentConfigs)

	it('creates a config lazily and keeps it', () => {
		expect(getAgentConfig('a-dev')).toBe(getAgentConfig('a-dev'))
	})

	it('derives the title from the agent the first time', () => {
		expect(
			getAgentConfig('a-cto', { role: 'CTO', model: 'x' }).fields.title,
		).toBe('CTO · Natalia')
	})

	it('updates and resets', () => {
		updateAgentConfig('a-dev', { paused: true })
		expect(getAgentConfig('a-dev').paused).toBe(true)
		resetAgentConfigs()
		expect(getAgentConfig('a-dev').paused).toBe(false)
	})
})
