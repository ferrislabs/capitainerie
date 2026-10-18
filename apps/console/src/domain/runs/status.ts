import type { Schemas } from '#/api/api.client'
import type { StatusTone } from '#/components/layout/status-badge'

export type RunStatus = Schemas.RunStatusResponse

export const STATUS_LABELS: Record<RunStatus, string> = {
	queued: 'En attente',
	running: 'En cours',
	succeeded: 'Terminé',
	failed: 'Échec',
	cancelled: 'Annulé',
}

export const STATUS_TONES: Record<RunStatus, StatusTone> = {
	queued: 'neutral',
	running: 'running',
	succeeded: 'success',
	failed: 'danger',
	cancelled: 'neutral',
}
