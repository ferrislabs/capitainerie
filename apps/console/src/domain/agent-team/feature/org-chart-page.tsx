import { DemoBanner } from '#/components/layout/demo-banner'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import {
	getAgentConfig,
	useConfigVersion,
} from '#/domain/agent-team/agent-config-store'
import { buildOrgTree } from '#/domain/agent-team/org-tree'
import { AgentsTabs } from '#/domain/agent-team/ui/agents-tabs'
import {
	type OrgEntry,
	OrgTreeView,
} from '#/domain/agent-team/ui/org-tree-view'
import { useDemoState } from '#/domain/demo'

export function OrgChartPage() {
	const state = useDemoState((s) => s)
	useConfigVersion()
	const entries = buildOrgTree(state).flatMap((node): OrgEntry[] => {
		const agent = state.agents.find((a) => a.id === node.id)
		if (!agent) return []
		return [
			{
				agent,
				depth: node.depth,
				subCount: node.subCount,
				canDelegate: getAgentConfig(agent.id).fields.canDeleg,
			},
		]
	})
	return (
		<Page>
			<PageHeader
				title="Organigramme"
				description="Qui rapporte à qui, de la direction jusqu'aux sous-agents."
			/>
			<DemoBanner />
			<AgentsTabs />
			<SectionCard title="Hiérarchie" flush>
				<OrgTreeView entries={entries} />
			</SectionCard>
		</Page>
	)
}
