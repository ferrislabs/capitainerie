import type { StatusTone } from '#/components/layout/status-badge'

export interface WorksiteStatus {
	label: string
	tone: StatusTone
}

/** L’état d’un chantier, lu sur sa dernière exécution puis sur sa colonne. */
export function worksiteStatus(item: {
	run_status?: string | null
	column: string
}): WorksiteStatus {
	switch (item.run_status) {
		case 'queued':
			return { label: 'En file', tone: 'neutral' }
		case 'running':
			return { label: 'En cours', tone: 'running' }
		case 'succeeded':
			return { label: 'À relire', tone: 'attention' }
		case 'failed':
			return { label: 'Échec', tone: 'danger' }
		case 'cancelled':
			return { label: 'Annulé', tone: 'neutral' }
	}
	return item.column === 'done'
		? { label: 'Terminé', tone: 'success' }
		: { label: 'À lancer', tone: 'neutral' }
}

export function hasActiveWorksite(
	items: ReadonlyArray<{ run_status?: string | null }> | undefined,
): boolean {
	return (
		items?.some(
			(item) => item.run_status === 'queued' || item.run_status === 'running',
		) ?? false
	)
}
