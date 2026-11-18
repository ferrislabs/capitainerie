export const CONNECTOR_TABS = [
	{ slug: 'installes', label: 'Installés' },
	{ slug: 'archives', label: 'Archivés' },
] as const

export type ConnectorTab = (typeof CONNECTOR_TABS)[number]['slug']

export function parseConnectorTab(raw: string | undefined): ConnectorTab {
	return CONNECTOR_TABS.find((t) => t.slug === raw)?.slug ?? 'installes'
}
