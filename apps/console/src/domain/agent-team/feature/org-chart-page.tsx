import { DemoBanner } from '#/components/layout/demo-banner'
import { Page } from '#/components/layout/page'
import {
	getAgentConfig,
	useConfigVersion,
} from '#/domain/agent-team/agent-config-store'
import { buildOrgTree } from '#/domain/agent-team/org-tree'
import { AgentsHeader } from '#/domain/agent-team/ui/agents-header'
import { AgentsTabs } from '#/domain/agent-team/ui/agents-tabs'
import {
	type OrgEntry,
	OrgTreeView,
} from '#/domain/agent-team/ui/org-tree-view'
import { Panel } from '#/domain/agent-team/ui/panel'
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
			<AgentsHeader />
			<DemoBanner />
			<AgentsTabs active="organigramme" />
			<Panel>
				<OrgTreeView entries={entries} />
			</Panel>
		</Page>
	)
}
