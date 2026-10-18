import { useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import {
	COSTS_WINDOW_DAYS,
	type CostDimension,
} from '#/domain/agents/dimensions'
import { useAgents } from '#/domain/agents/hooks/use-agents'
import { useCosts } from '#/domain/agents/hooks/use-costs'
import { AgentsTable } from '#/domain/agents/ui/agents-table'
import { CostsSection } from '#/domain/agents/ui/costs-section'

export function AgentsPage() {
	const [by, setBy] = useState<CostDimension>('agent')
	const agents = useAgents()
	const costs = useCosts(by)

	return (
		<Page>
			<PageHeader
				title="Agents"
				description="Chaque agent agit avec un jeton délégué. Son activité et ce qu'elle coûte, run par run."
			/>
			<SectionCard
				title="Agents actifs"
				description="Runs, dernière activité, tokens et coût cumulés."
				bare
			>
				{agents.isError ? (
					<InlineAlert tone="danger">
						Impossible de charger les agents.
					</InlineAlert>
				) : agents.isPending ? (
					<TableSkeleton rows={3} columns={5} />
				) : (
					<AgentsTable agents={agents.data} />
				)}
			</SectionCard>
			<CostsSection
				title={`Coûts des ${COSTS_WINDOW_DAYS} derniers jours`}
				by={by}
				costs={costs.data}
				isPending={costs.isPending}
				isError={costs.isError}
				onChange={setBy}
			/>
		</Page>
	)
}
