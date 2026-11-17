import { Store } from '@tanstack/store'
import { useSyncExternalStore } from 'react'
import { DEMO_AGENTS, shortName } from '#/domain/demo/agents'
import { DEMO_CONNECTORS, DEMO_GRANTS } from '#/domain/demo/connectors'
import { DEMO_SOURCES } from '#/domain/demo/sources'
import type { DemoAgent, DemoSource } from '#/domain/demo/types'

/**
 * État partagé des pages de démonstration : ce que plusieurs pages doivent voir
 * de la même façon (autorisations de connecteurs, hiérarchie, sources).
 * En mémoire seulement : perdu au rechargement.
 */
export interface DemoState {
	/** Équipe : les agents de départ plus ceux créés pendant la session. */
	agents: ReadonlyArray<DemoAgent>
	/** Identifiant d'agent vers noms de connecteurs autorisés. Refus par défaut. */
	grants: Readonly<Record<string, ReadonlyArray<string>>>
	/** Identifiant d'agent vers nom court de l'agent dont il dépend, `null` pour la direction. */
	reportsTo: Readonly<Record<string, string | null>>
	sources: ReadonlyArray<DemoSource>
}

function initialState(): DemoState {
	return {
		agents: DEMO_AGENTS,
		grants: DEMO_GRANTS,
		reportsTo: Object.fromEntries(DEMO_AGENTS.map((a) => [a.id, a.reportsTo])),
		sources: DEMO_SOURCES,
	}
}

export const demoStore = new Store<DemoState>(initialState())

export function resetDemoState(): void {
	demoStore.setState(() => initialState())
}

export function useDemoState<T>(select: (state: DemoState) => T): T {
	return useSyncExternalStore(
		(onChange) => demoStore.subscribe(onChange).unsubscribe,
		() => select(demoStore.state),
		() => select(initialState()),
	)
}

/** Un connecteur sans compte branché ne s'autorise à personne : l'appel est ignoré. */
export function grantConnector(agentId: string, connector: string): boolean {
	const usable = DEMO_CONNECTORS.some(
		(c) => c.name === connector && c.accounts.length > 0,
	)
	if (!usable) return false
	demoStore.setState((state) => {
		const current = state.grants[agentId] ?? []
		if (current.includes(connector)) return state
		return {
			...state,
			grants: { ...state.grants, [agentId]: [...current, connector] },
		}
	})
	return true
}

export function revokeConnector(agentId: string, connector: string): void {
	demoStore.setState((state) => ({
		...state,
		grants: {
			...state.grants,
			[agentId]: (state.grants[agentId] ?? []).filter((c) => c !== connector),
		},
	}))
}

export function addAgent(agent: DemoAgent): void {
	demoStore.setState((state) => ({
		...state,
		agents: [...state.agents, agent],
		reportsTo: { ...state.reportsTo, [agent.id]: agent.reportsTo },
	}))
}

export function updateAgent(
	id: string,
	patch: Partial<Omit<DemoAgent, 'id'>>,
): void {
	demoStore.setState((state) => ({
		...state,
		agents: state.agents.map((a) => (a.id === id ? { ...a, ...patch } : a)),
	}))
}

export function setReportsTo(agentId: string, parent: string | null): void {
	demoStore.setState((state) => ({
		...state,
		reportsTo: { ...state.reportsTo, [agentId]: parent },
	}))
}

function patchSource(id: string, patch: Partial<DemoSource>) {
	demoStore.setState((state) => ({
		...state,
		sources: state.sources.map((s) => (s.id === id ? { ...s, ...patch } : s)),
	}))
}

export const updateSource = patchSource

export function addSource(source: DemoSource): void {
	demoStore.setState((state) => ({
		...state,
		sources: [...state.sources, source],
	}))
}

export function attachSource(sourceId: string, agentId: string): void {
	const source = demoStore.state.sources.find((s) => s.id === sourceId)
	if (source && !source.agentIds.includes(agentId))
		patchSource(sourceId, { agentIds: [...source.agentIds, agentId] })
}

export function detachSource(sourceId: string, agentId: string): void {
	const source = demoStore.state.sources.find((s) => s.id === sourceId)
	if (source)
		patchSource(sourceId, {
			agentIds: source.agentIds.filter((id) => id !== agentId),
		})
}

/** Agents qui dépendent directement de `agentId`. */
export function subAgentIds(state: DemoState, agentId: string): string[] {
	const parent = state.agents.find((a) => a.id === agentId)
	if (!parent) return []
	const name = shortName(parent.name)
	return state.agents
		.filter((a) => a.id !== agentId && state.reportsTo[a.id] === name)
		.map((a) => a.id)
}

/** Chaîne hiérarchique au-dessus de `agentId`, du parent direct vers la direction. */
export function ancestorIds(state: DemoState, agentId: string): string[] {
	const chain: string[] = []
	let name = state.reportsTo[agentId] ?? null
	while (name && chain.length < 16) {
		const parent = state.agents.find((a) => shortName(a.name) === name)
		if (!parent || parent.id === agentId || chain.includes(parent.id)) break
		chain.push(parent.id)
		name = state.reportsTo[parent.id] ?? null
	}
	return chain
}

export function agentsWithGrant(state: DemoState, connector: string): string[] {
	return state.agents
		.filter((a) => (state.grants[a.id] ?? []).includes(connector))
		.map((a) => a.id)
}
