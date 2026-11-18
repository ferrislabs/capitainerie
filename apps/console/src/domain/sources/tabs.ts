export const SOURCE_TABS = [
	{ id: 'apercu', label: "Vue d'ensemble" },
	{ id: 'evenements', label: 'Événements' },
	{ id: 'parametres', label: 'Paramètres' },
] as const

export type SourceTabId = (typeof SOURCE_TABS)[number]['id']

export function resolveTab(tab: string | undefined): SourceTabId {
	return SOURCE_TABS.find((t) => t.id === tab)?.id ?? 'apercu'
}
