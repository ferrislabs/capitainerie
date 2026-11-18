import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { Skeleton } from '#/components/ui/skeleton'
import { parseConfig } from '#/domain/agent-team/agent-config'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { buildOrgTree } from '#/domain/agent-team/org-tree'
import { AgentsHeader } from '#/domain/agent-team/ui/agents-header'
import { AgentsTabs } from '#/domain/agent-team/ui/agents-tabs'
import {
	type OrgEntry,
	OrgTreeView,
} from '#/domain/agent-team/ui/org-tree-view'
import { Panel } from '#/domain/agent-team/ui/panel'
import { Empty } from '#/domain/agent-team/ui/rows'

export function OrgChartPage() {
	const query = useTeamAgents(false)
	const agents = query.data?.items ?? []
	const entries = buildOrgTree(agents).flatMap((node): OrgEntry[] => {
		const agent = agents.find((a) => a.id === node.id)
		if (!agent) return []
		return [
			{
				agent,
				depth: node.depth,
				subCount: node.subCount,
				canDelegate: parseConfig(agent).canDeleg,
			},
		]
	})
	return (
		<Page>
			<AgentsHeader />
			<AgentsTabs active="organigramme" />
			<Panel>
				{query.isError ? (
					<InlineAlert tone="danger">
						Impossible de charger l'organigramme.
					</InlineAlert>
				) : query.isPending ? (
					<output className="grid gap-2 p-3">
						<span className="sr-only">Chargement…</span>
						<Skeleton className="h-9" />
						<Skeleton className="h-9" />
					</output>
				) : entries.length === 0 ? (
					<Empty title="Aucun agent">
						Crée un agent pour construire l'organigramme.
					</Empty>
				) : (
					<OrgTreeView entries={entries} />
				)}
			</Panel>
		</Page>
	)
}
