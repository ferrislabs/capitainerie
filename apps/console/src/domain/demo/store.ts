import { Store } from '@tanstack/store'
import { useSyncExternalStore } from 'react'
import { DEMO_AGENTS, shortName } from '#/domain/demo/agents'
import { DEMO_APPROVALS } from '#/domain/demo/approvals'
import { DEMO_CONNECTORS, DEMO_GRANTS } from '#/domain/demo/connectors'
import { DEMO_SOURCES } from '#/domain/demo/sources'
import { DEMO_TASKS } from '#/domain/demo/tasks'
import type {
	DemoAgent,
	DemoApproval,
	DemoApprovalStatus,
	DemoSource,
	DemoTask,
} from '#/domain/demo/types'

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
	tasks: ReadonlyArray<DemoTask>
	approvals: ReadonlyArray<DemoApproval>
}

function initialState(): DemoState {
	return {
		agents: DEMO_AGENTS,
		grants: DEMO_GRANTS,
		reportsTo: Object.fromEntries(DEMO_AGENTS.map((a) => [a.id, a.reportsTo])),
		sources: DEMO_SOURCES,
		tasks: DEMO_TASKS,
		approvals: DEMO_APPROVALS,
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

export function addApproval(approval: DemoApproval): void {
	demoStore.setState((state) => ({
		...state,
		approvals: [approval, ...state.approvals],
	}))
}

export function decideApproval(
	id: string,
	status: Exclude<DemoApprovalStatus, 'pending'>,
	decidedBy: string,
): void {
	demoStore.setState((state) => ({
		...state,
		approvals: state.approvals.map((a) =>
			a.id === id && a.status === 'pending'
				? { ...a, status, decidedBy, at: "à l'instant" }
				: a,
		),
	}))
}
