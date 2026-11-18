/**
 * Données de démonstration partagées par les pages sans backend.
 * Figées : aucune page ne les modifie, elle copie ce dont elle a besoin dans son état local.
 */

export type DemoAgentStatus = 'running' | 'idle'

export interface DemoAgent {
	id: string
	name: string
	role: string
	description: string
	status: DemoAgentStatus
	hue: number
	model: string
	/** Nom court de l'agent dont il dépend (hiérarchie), `null` pour la direction. */
	reportsTo: string | null
}

export interface DemoProject {
	name: string
	description: string
	progress: number
	openTasks: number
	agentId: string
}

export type DemoTaskStatus =
	| 'todo'
	| 'progress'
	| 'review'
	| 'done'
	| 'blocked'
	| 'cancelled'
	| 'backlog'

export interface DemoTask {
	id: string
	title: string
	status: DemoTaskStatus
	agentId: string
	age: string
	project: string
	note?: string
	parentId?: string
}
