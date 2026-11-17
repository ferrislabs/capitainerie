export const SOURCE_TABS = [
	{ id: 'apercu', label: "Vue d'ensemble" },
	{ id: 'regles', label: 'Motifs et exclusions' },
	{ id: 'pipeline', label: 'Pipeline' },
	{ id: 'evenements', label: 'Événements' },
	{ id: 'agents', label: 'Agents' },
] as const

export type SourceTabId = (typeof SOURCE_TABS)[number]['id']

export function resolveTab(tab: string | undefined): SourceTabId {
	return SOURCE_TABS.find((t) => t.id === tab)?.id ?? 'apercu'
}
