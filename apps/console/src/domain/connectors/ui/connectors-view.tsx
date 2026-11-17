import { DemoBanner } from '#/components/layout/demo-banner'
import { Notice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { TabNav } from '#/components/layout/tab-nav'
import { Button } from '#/components/ui/button'
import { HumanAvatar } from '#/domain/audit/ui/avatar'
import { Panel, PanelEmpty, Row, Rows, RowTitle } from '#/domain/audit/ui/rows'
import { CONNECTOR_TABS } from '#/domain/connectors/tabs'
import { ConnectorCard } from '#/domain/connectors/ui/connector-card'
import type { DemoAgent, DemoConnector } from '#/domain/demo'

interface Props {
	connectors: ReadonlyArray<DemoConnector>
	agents: ReadonlyArray<DemoAgent>
	grantedIds: (connector: string) => ReadonlyArray<string>
	notice: string | null
	onConfigure: () => void
	onAddAccount: (connector: string) => void
	onGrant: (agentId: string, connector: string) => void
	onRevoke: (agentId: string, connector: string) => void
}

export function ConnectorsView({
	connectors,
	agents,
	grantedIds,
	notice,
	onConfigure,
	onAddAccount,
	onGrant,
	onRevoke,
}: Props) {
	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Connecteurs"
				description="Canaux, comptes, outils MCP et registres branchés, puis ouverts aux agents"
			/>
			<Notice message={notice} />
			<Panel className="mb-3">
				<Rows>
					<Row>
						<HumanAvatar initials="MCP" size={30} />
						<RowTitle
							title="Connexion d'assistant (MCP)"
							sub="Utilise ton organisation depuis Codex, Claude, OpenCode ou un autre assistant."
						/>
						<Button variant="outline" onClick={onConfigure}>
							Configurer
						</Button>
					</Row>
				</Rows>
			</Panel>
			<TabNav
				label="Connecteurs"
				tabs={CONNECTOR_TABS.map((t) => ({
					label: t.label,
					link: { to: '/connectors/{-$tab}', params: { tab: t.slug } },
				}))}
			/>
			{connectors.length === 0 ? (
				<PanelEmpty title="Aucun connecteur" />
			) : (
				<div className="grid grid-cols-1 gap-2.5 md:grid-cols-2">
					{connectors.map((c) => (
						<ConnectorCard
							key={c.name}
							connector={c}
							agents={agents}
							grantedIds={grantedIds(c.name)}
							onAddAccount={onAddAccount}
							onGrant={onGrant}
							onRevoke={onRevoke}
						/>
					))}
				</div>
			)}
		</Page>
	)
}
