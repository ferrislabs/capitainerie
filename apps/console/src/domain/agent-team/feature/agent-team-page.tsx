import { Link } from '@tanstack/react-router'
import { Plus } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { AgentList } from '#/domain/agent-team/ui/agent-list'
import { AgentsTabs } from '#/domain/agent-team/ui/agents-tabs'
import { useDemoState } from '#/domain/demo'

export function AgentTeamPage() {
	const agents = useDemoState((s) => s.agents)
	const running = agents.filter((a) => a.status === 'running').length
	return (
		<Page>
			<PageHeader
				title="Équipe"
				description={`${agents.length} agents · ${running} en cours`}
				actions={
					<Button asChild>
						<Link to="/agents/nouveau">
							<Plus aria-hidden="true" />
							Nouvel agent
						</Link>
					</Button>
				}
			/>
			<DemoBanner />
			<AgentsTabs />
			<SectionCard title="Agents" flush>
				<AgentList agents={agents} />
			</SectionCard>
		</Page>
	)
}
