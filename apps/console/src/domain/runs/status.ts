import type { Schemas } from '#/api/api.client'

export type RunStatus = Schemas.RunStatusResponse
export type BadgeVariant = 'muted' | 'default' | 'success' | 'destructive'

export const STATUS_LABELS: Record<RunStatus, string> = {
	queued: 'En attente',
	running: 'En cours',
	succeeded: 'Terminé',
	failed: 'Échec',
	cancelled: 'Annulé',
}

export const STATUS_VARIANTS: Record<RunStatus, BadgeVariant> = {
	queued: 'muted',
	running: 'default',
	succeeded: 'success',
	failed: 'destructive',
	cancelled: 'muted',
}
