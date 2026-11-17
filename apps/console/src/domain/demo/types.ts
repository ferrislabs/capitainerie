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

export type DemoConnectorKind = 'Canal' | 'MCP' | 'Outil' | 'Registre'

export type DemoToolRisk =
	| 'lecture'
	| 'écriture limitée'
	| 'écriture'
	| 'envoi externe'

export interface DemoTool {
	name: string
	capability: string
	risk: DemoToolRisk
}

export interface DemoConnector {
	name: string
	kind: DemoConnectorKind
	description: string
	/** Comptes branchés ; vide : le connecteur ne peut être autorisé à aucun agent. */
	accounts: ReadonlyArray<string>
	tools: ReadonlyArray<DemoTool>
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
	/** Identifiant de la source qui a déclenché la tâche. */
	sourceId?: string
	/** Valeur reçue de la source (nom d'hôte, adresse). */
	sourceValue?: string
	/** Dossier produit par un agent de recherche : paires libellé, valeur. */
	dossier?: ReadonlyArray<readonly [string, string]>
}

export type DemoSourceType = 'DNS' | 'Webhook' | 'Mail' | 'RSS' | 'Git' | 'Odoo'

export interface DemoSource {
	id: string
	name: string
	type: DemoSourceType
	/** Motifs écoutés : `*` remplace n'importe quels caractères. */
	patterns: ReadonlyArray<string>
	/** Exclusions : prioritaires sur les motifs. */
	exclusions: ReadonlyArray<string>
	feeds: ReadonlyArray<string>
	scope: string
	mode: string
	dedupe: string
	/** Titre de la tâche créée, avec une variable entre accolades. */
	template: string
	agentIds: ReadonlyArray<string>
	enabled: boolean
	eventsLast24h: number
	last: string
}

export type DemoApprovalStatus = 'pending' | 'approved' | 'rejected'

export interface DemoApproval {
	id: string
	title: string
	detail: string
	/** Agent qui demande la décision, `null` si elle vient du système. */
	requestedBy: string | null
	status: DemoApprovalStatus
	/** Date lisible : « 12 nov. », « à l'instant ». */
	at: string
	/** Qui a tranché, vide tant que la demande est en attente. */
	decidedBy: string
	/** Tâche concernée, s'il y en a une. */
	taskId?: string
}
