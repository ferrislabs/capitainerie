import { useSyncExternalStore } from 'react'
import {
	type AgentConfig,
	buildDefaultConfig,
} from '#/domain/agent-team/agent-config'
import { demoStore } from '#/domain/demo'

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

export function getAgentConfig(id: string): AgentConfig {
	const existing = configs.get(id)
	if (existing) return existing
	const agent = demoStore.state.agents.find((a) => a.id === id) ?? {
		id,
		name: id,
		role: 'Général',
		description: '',
		status: 'idle' as const,
		hue: 0,
		model: 'Claude Sonnet 5.5',
		reportsTo: null,
	}
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

export function useAgentConfig(id: string): AgentConfig {
	return useSyncExternalStore(
		subscribe,
		() => getAgentConfig(id),
		() => getAgentConfig(id),
	)
}

export function useConfigVersion(): number {
	return useSyncExternalStore(
		subscribe,
		() => version,
		() => version,
	)
}
