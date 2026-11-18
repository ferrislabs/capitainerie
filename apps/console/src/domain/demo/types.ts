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
