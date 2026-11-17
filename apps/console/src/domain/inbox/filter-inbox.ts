import type { InboxTab } from '#/domain/inbox/tabs'

export type InboxKind = 'task' | 'run' | 'mr' | 'routine'

export interface InboxItem {
	id: string
	kind: InboxKind
	ref: string
	title: string
	source: string
	when: string
	unread: boolean
	scope: 'moi' | 'recent'
}

export function filterInbox(
	tab: InboxTab,
	items: ReadonlyArray<InboxItem>,
): InboxItem[] {
	switch (tab) {
		case 'tous':
			return [...items]
		case 'non-lus':
			return items.filter((item) => item.unread)
		case 'moi':
		case 'recent':
			return items.filter((item) => item.scope === tab)
	}
}
