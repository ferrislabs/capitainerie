import { Plug } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { EmptyState } from '#/components/layout/empty-state'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { TabNav } from '#/components/layout/tab-nav'
import { Button } from '#/components/ui/button'
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
			<p aria-live="polite" className="min-h-5 text-muted-foreground">
				{notice}
			</p>
			<SectionCard
				title="Connexion d'assistant (MCP)"
				description="Utilise ton organisation depuis Codex, Claude, OpenCode ou un autre assistant."
				actions={
					<Button variant="outline" size="sm" onClick={onConfigure}>
						Configurer
					</Button>
				}
			>
				{null}
			</SectionCard>
			<TabNav
				label="Connecteurs"
				tabs={CONNECTOR_TABS.map((t) => ({
					label: t.label,
					link:
						t.slug === 'installes'
							? { to: '/connectors/{-$tab}' }
							: { to: '/connectors/{-$tab}', params: { tab: t.slug } },
				}))}
			/>
			{connectors.length === 0 ? (
				<EmptyState
					icon={Plug}
					title="Aucun connecteur"
					description="Aucun connecteur ne correspond à cet onglet."
				/>
			) : (
				<div className="grid gap-3 md:grid-cols-2">
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
