import { beforeEach, describe, expect, it } from 'vitest'
import {
	getAgentConfig,
	resetAgentConfigs,
	updateAgentConfig,
} from '#/domain/agent-team/agent-config-store'

describe('agent config store', () => {
	beforeEach(resetAgentConfigs)

	it('creates a config lazily and keeps it', () => {
		expect(getAgentConfig('dev')).toBe(getAgentConfig('dev'))
	})

	it('updates and resets', () => {
		updateAgentConfig('dev', { paused: true })
		expect(getAgentConfig('dev').paused).toBe(true)
		resetAgentConfigs()
		expect(getAgentConfig('dev').paused).toBe(false)
	})
})
