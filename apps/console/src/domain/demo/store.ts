import { Store } from '@tanstack/store'
import { useSyncExternalStore } from 'react'
import { DEMO_AGENTS } from '#/domain/demo/agents'
import { DEMO_GRANTS } from '#/domain/demo/connectors'
import { DEMO_TASKS } from '#/domain/demo/tasks'
import type { DemoAgent, DemoTask } from '#/domain/demo/types'

/**
 * État partagé des pages de démonstration restantes (routines) : équipe,
 * autorisations de connecteurs et tâches. En mémoire seulement : perdu au
 * rechargement.
 */
export interface DemoState {
	agents: ReadonlyArray<DemoAgent>
	/** Identifiant d'agent vers noms de connecteurs autorisés. Refus par défaut. */
	grants: Readonly<Record<string, ReadonlyArray<string>>>
	tasks: ReadonlyArray<DemoTask>
}

function initialState(): DemoState {
	return { agents: DEMO_AGENTS, grants: DEMO_GRANTS, tasks: DEMO_TASKS }
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
