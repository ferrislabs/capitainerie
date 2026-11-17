export type EventDecision =
	| 'done'
	| 'progress'
	| 'review'
	| 'ignored'
	| 'dup'
	| 'none'

export interface SourceEvent {
	receivedAt: string
	value: string
	pattern: string
	decision: EventDecision
	detail: string
}

export const DECISION_LABELS: Record<
	EventDecision,
	{ label: string; tone: 'ok' | 'warn' | 'info' }
> = {
	done: { label: 'Tâche créée', tone: 'ok' },
	progress: { label: 'En cours', tone: 'info' },
	review: { label: 'À valider', tone: 'warn' },
	ignored: { label: 'Ignoré', tone: 'info' },
	dup: { label: 'Doublon', tone: 'info' },
	none: { label: 'Sans correspondance', tone: 'info' },
}

const ev = (
	receivedAt: string,
	value: string,
	pattern: string,
	decision: EventDecision,
	detail: string,
): SourceEvent => ({ receivedAt, value, pattern, decision, detail })

const EVENTS: Readonly<Record<string, ReadonlyArray<SourceEvent>>> = {
	'nouveaux-sous-domaines-sensibles': [
		ev('il y a 4 min', 'iam.acme-logistique.fr', 'iam.*', 'done', 'NAT-690'),
		ev(
			'il y a 31 min',
			'auth.lumen-sante.com',
			'auth.*',
			'progress',
			'NAT-691',
		),
		ev(
			'il y a 1 h',
			'auth.getnatalia.com',
			'',
			'ignored',
			'Exclu : *.getnatalia.com',
		),
		ev(
			'il y a 2 h',
			'iam.acme-logistique.fr',
			'iam.*',
			'dup',
			'Déjà traité : NAT-690',
		),
		ev('il y a 3 h', 'iam.novabank-groupe.eu', 'iam.*', 'review', 'NAT-689'),
		ev(
			'il y a 5 h',
			'authentik.exemple.org',
			'',
			'none',
			'Aucun motif ne correspond',
		),
		ev(
			'il y a 7 h',
			'login.acme-logistique.fr',
			'',
			'none',
			'Aucun motif ne correspond',
		),
	],
	'pieces-recues-sur-chatwoot': [
		ev(
			'il y a 2 h',
			'conversation 550, message 1990',
			'pièce jointe',
			'done',
			'NAT-684',
		),
		ev(
			'il y a 3 h',
			'conversation 497, message 1989',
			'pièce jointe',
			'done',
			'NAT-683',
		),
		ev(
			'il y a 4 h',
			'conversation 497, message 1982',
			'pièce jointe',
			'done',
			'NAT-682',
		),
	],
	'boite-du-cabinet-comptable': [
		ev(
			'il y a 31 min',
			'Relevé de février.pdf',
			'cabinet@natalia.fr',
			'done',
			'',
		),
		ev('il y a 2 h', 'Facture 2026-1187', 'cabinet@natalia.fr', 'done', ''),
	],
}

export function eventsFor(sourceId: string): ReadonlyArray<SourceEvent> {
	return EVENTS[sourceId] ?? []
}
