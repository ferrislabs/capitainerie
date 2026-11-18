import { useSyncExternalStore } from 'react'
import {
	type AgentConfig,
	buildDefaultConfig,
} from '#/domain/agent-team/agent-config'
import type { TeamAgent } from '#/domain/agent-team/team-agent'

const configs = new Map<string, AgentConfig>()
const listeners = new Set<() => void>()
let version = 0

function emit() {
	version++
	for (const listener of listeners) listener()
}

function subscribe(listener: () => void) {
	listeners.add(listener)
	return () => {
		listeners.delete(listener)
	}
}

const FALLBACK_AGENT = { role: 'Général', model: '' }

export function getAgentConfig(
	id: string,
	agent: Pick<TeamAgent, 'role' | 'model'> = FALLBACK_AGENT,
): AgentConfig {
	const existing = configs.get(id)
	if (existing) return existing
	const created = buildDefaultConfig(agent)
	configs.set(id, created)
	return created
}

export function seedAgentConfig(id: string, config: AgentConfig): void {
	configs.set(id, config)
	emit()
}

export function updateAgentConfig(
	id: string,
	patch: Partial<AgentConfig>,
): void {
	configs.set(id, { ...getAgentConfig(id), ...patch })
	emit()
}

export function resetAgentConfigs(): void {
	configs.clear()
	emit()
}

export function useAgentConfig(
	id: string,
	agent?: Pick<TeamAgent, 'role' | 'model'>,
): AgentConfig {
	return useSyncExternalStore(
		subscribe,
		() => getAgentConfig(id, agent),
		() => getAgentConfig(id, agent),
	)
}

export function useConfigVersion(): number {
	return useSyncExternalStore(
		subscribe,
		() => version,
		() => version,
	)
}
