import type { DemoConnector } from '#/domain/demo'

export const CONNECTOR_TABS = [
	{ slug: 'installes', label: 'Installés' },
	{ slug: 'canaux', label: 'Canaux' },
	{ slug: 'outils', label: 'Outils' },
	{ slug: 'registres', label: 'Registres' },
	{ slug: 'tous', label: 'Tous' },
] as const

export type ConnectorTab = (typeof CONNECTOR_TABS)[number]['slug']

export function parseConnectorTab(raw: string | undefined): ConnectorTab {
	return CONNECTOR_TABS.find((t) => t.slug === raw)?.slug ?? 'installes'
}

export function filterConnectors(
	tab: ConnectorTab,
	connectors: ReadonlyArray<DemoConnector>,
): DemoConnector[] {
	switch (tab) {
		case 'installes':
			return connectors.filter((c) => c.accounts.length > 0)
		case 'canaux':
			return connectors.filter((c) => c.kind === 'Canal')
		case 'outils':
			return connectors.filter((c) => c.kind === 'Outil' || c.kind === 'MCP')
		case 'registres':
			return connectors.filter((c) => c.kind === 'Registre')
		case 'tous':
			return [...connectors]
	}
}
