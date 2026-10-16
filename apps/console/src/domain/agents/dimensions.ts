import type { Endpoints } from '#/api/api.client'

export type CostDimension = NonNullable<
	Endpoints.get_GetCosts['parameters']['query']['by']
>

export const COSTS_WINDOW_DAYS = 30

export const DIMENSIONS: ReadonlyArray<{
	value: CostDimension
	label: string
	keyLabel: string
}> = [
	{ value: 'agent', label: 'Par agent', keyLabel: 'Agent' },
	{ value: 'user', label: 'Par utilisateur', keyLabel: 'Utilisateur' },
	{ value: 'model', label: 'Par modèle', keyLabel: 'Modèle' },
	{ value: 'day', label: 'Par jour', keyLabel: 'Jour' },
]

export function costsWindow(now: Date): { from: string; to: string } {
	const from = new Date(now.getTime() - COSTS_WINDOW_DAYS * 86_400_000)
	return { from: from.toISOString(), to: now.toISOString() }
}
