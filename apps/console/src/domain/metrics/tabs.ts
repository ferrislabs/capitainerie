export const METRICS_TABS = [
	{ slug: 'projets', label: 'Projets' },
	{ slug: 'agents', label: 'Agents' },
	{ slug: 'taches', label: 'Tâches' },
	{ slug: 'utilisateurs', label: 'Utilisateurs' },
] as const

export type MetricsTab = (typeof METRICS_TABS)[number]['slug']

export function parseMetricsTab(value: string | undefined): MetricsTab {
	return METRICS_TABS.find((tab) => tab.slug === value)?.slug ?? 'projets'
}
