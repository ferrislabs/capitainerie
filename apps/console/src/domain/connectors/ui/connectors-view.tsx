import { Plus } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Notice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { TabNav } from '#/components/layout/tab-nav'
import { Button } from '#/components/ui/button'
import { PanelEmpty } from '#/domain/audit/ui/rows'
import { CONNECTOR_TABS } from '#/domain/connectors/tabs'
import {
	ConnectorCard,
	type ToolsState,
} from '#/domain/connectors/ui/connector-card'

interface Props {
	connectors: ReadonlyArray<Schemas.McpConnectorResponse>
	loading: boolean
	error: boolean
	notice: string | null
	banner: string | null
	busyIds: ReadonlyArray<string>
	tools: Readonly<Record<string, ToolsState>>
	onAdd: () => void
	onRetry: () => void
	onConnect: (id: string) => void
	onDisconnect: (id: string) => void
	onTest: (id: string) => void
	onArchive: (id: string, archived: boolean) => void
}

export function ConnectorsView({
	connectors,
	loading,
	error,
	notice,
	banner,
	busyIds,
	tools,
	onAdd,
	onRetry,
	onConnect,
	onDisconnect,
	onTest,
	onArchive,
}: Props) {
	return (
		<Page>
			<PageHeader
				title="Connecteurs"
				description="Serveurs MCP branchés avec ton compte (OAuth2), puis ouverts aux agents"
				actions={
					<Button onClick={onAdd}>
						<Plus aria-hidden="true" />
						Ajouter un connecteur
					</Button>
				}
			/>
			{banner ? (
				<InlineAlert tone="success" className="mb-3">
					{banner}
				</InlineAlert>
			) : null}
			<Notice message={notice} className="mb-3" />
			<TabNav
				label="Connecteurs"
				tabs={CONNECTOR_TABS.map((t) => ({
					label: t.label,
					link: { to: '/connectors/{-$tab}', params: { tab: t.slug } },
				}))}
			/>
			{error ? (
				<div className="grid gap-2">
					<InlineAlert tone="danger">
						Impossible de charger les connecteurs.
					</InlineAlert>
					<div>
						<Button variant="outline" onClick={onRetry}>
							Réessayer
						</Button>
					</div>
				</div>
			) : loading ? (
				<output className="m-0 text-muted-foreground">
					Chargement des connecteurs…
				</output>
			) : connectors.length === 0 ? (
				<PanelEmpty title="Aucun connecteur" />
			) : (
				<div className="grid grid-cols-1 gap-2.5 md:grid-cols-2">
					{connectors.map((c) => (
						<ConnectorCard
							key={c.id}
							connector={c}
							busy={busyIds.includes(c.id)}
							tools={tools[c.id]}
							onConnect={onConnect}
							onDisconnect={onDisconnect}
							onTest={onTest}
							onArchive={onArchive}
						/>
					))}
				</div>
			)}
		</Page>
	)
}
