import type { Schemas } from '#/api/api.client'

export type InboxItem = Schemas.InboxItemResponse
export type InboxKind = Schemas.InboxKindDto

export const KIND_LABEL: Readonly<Record<InboxKind, string>> = {
	approval: 'Approbation',
	run: 'Exécution',
	card: 'Tâche',
	source: 'Source',
	routine: 'Routine',
}

export type InboxTarget =
	| { kind: 'task'; taskId: string }
	| {
			kind: 'page'
			to: '/approvals' | '/routines' | '/sources' | '/audit/{-$tab}' | '/tasks'
	  }
	| { kind: 'internal'; href: string }
	| { kind: 'external'; href: string }

const isExternal = (link: string): boolean => /^https?:\/\//i.test(link)

/** Où mène un élément : la tâche liée, puis le lien fourni, puis la page de son type. */
export function targetOf(item: InboxItem): InboxTarget {
	if (item.card_id) return { kind: 'task', taskId: item.card_id }
	const link = item.link?.trim()
	if (link?.startsWith('/') && !link.startsWith('//'))
		return { kind: 'internal', href: link }
	if (link && isExternal(link)) return { kind: 'external', href: link }
	switch (item.kind) {
		case 'approval':
			return { kind: 'page', to: '/approvals' }
		case 'routine':
			return { kind: 'page', to: '/routines' }
		case 'source':
			return { kind: 'page', to: '/sources' }
		case 'run':
			return { kind: 'page', to: '/audit/{-$tab}' }
		case 'card':
			return { kind: 'page', to: '/tasks' }
	}
}
