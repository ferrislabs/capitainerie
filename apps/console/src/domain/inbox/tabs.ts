export const INBOX_TABS = [
	{ slug: 'tous', label: 'Tous' },
	{ slug: 'non-lus', label: 'Non lus' },
] as const

export type InboxTab = (typeof INBOX_TABS)[number]['slug']

export function parseInboxTab(value: string | undefined): InboxTab {
	return INBOX_TABS.find((tab) => tab.slug === value)?.slug ?? 'tous'
}
