import { Link } from '@tanstack/react-router'
import { Bot, SearchX } from 'lucide-react'
import { useMemo, useState } from 'react'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import { Button } from '#/components/ui/button'
import {
	COSTS_WINDOW_DAYS,
	type CostDimension,
} from '#/domain/agents/dimensions'
import {
	type AgentFilters,
	type AgentRow,
	buildRows,
	filterRows,
	NO_FILTERS,
	optionsOf,
	pruneFilters,
} from '#/domain/agents/filters'
import { useAgents } from '#/domain/agents/hooks/use-agents'
import { useCosts } from '#/domain/agents/hooks/use-costs'
import { AgentSheet } from '#/domain/agents/ui/agent-sheet'
import { AgentsTable } from '#/domain/agents/ui/agents-table'
import { AgentsToolbar } from '#/domain/agents/ui/agents-toolbar'
import { CostsSection } from '#/domain/agents/ui/costs-section'
import { useModelAssignments } from '#/domain/settings/hooks/use-settings-queries'

export function AgentsPage() {
	const [by, setBy] = useState<CostDimension>('agent')
	const [chosen, setFilters] = useState<AgentFilters>(NO_FILTERS)
	const [openId, setOpenId] = useState<string>()
	const agents = useAgents()
	const assignments = useModelAssignments()
	const costs = useCosts(by)

	const rows = useMemo(
		() => buildRows(agents.data ?? [], assignments.data?.assignments),
		[agents.data, assignments.data],
	)
	const filters = useMemo(() => pruneFilters(chosen, rows), [chosen, rows])
	const visible = useMemo(() => filterRows(rows, filters), [rows, filters])
	const opened: AgentRow | undefined = rows.find(
		(row) => row.agent.agent === openId,
	)

	return (
		<Page>
			<PageHeader
				title="Agents"
				description="Chaque agent agit avec un jeton délégué. Son activité et ce qu'elle coûte, run par run."
			/>
			{agents.isError ? (
				<InlineAlert tone="danger">
					Impossible de charger les agents.
				</InlineAlert>
			) : agents.isPending ? (
				<TableSkeleton rows={3} columns={6} />
			) : rows.length === 0 ? (
				<EmptyState
					icon={Bot}
					title="Aucun agent"
					description="Les agents apparaîtront ici après leur premier run."
					action={
						<Button asChild variant="outline">
							<Link to="/runs">Déposer une réunion</Link>
						</Button>
					}
				/>
			) : (
				<div className="flex min-w-0 flex-col gap-3">
					<AgentsToolbar
						filters={filters}
						sources={optionsOf(rows, 'source')}
						models={optionsOf(rows, 'model')}
						onChange={setFilters}
					/>
					{visible.length === 0 ? (
						<EmptyState
							icon={SearchX}
							title="Aucun agent ne correspond"
							description="Aucun agent ne répond à ces filtres."
							action={
								<Button
									type="button"
									variant="outline"
									onClick={() => setFilters(NO_FILTERS)}
								>
									Réinitialiser les filtres
								</Button>
							}
						/>
					) : (
						<AgentsTable
							rows={visible}
							onOpen={(row) => setOpenId(row.agent.agent)}
						/>
					)}
				</div>
			)}
			<CostsSection
				title={`Coûts des ${COSTS_WINDOW_DAYS} derniers jours`}
				by={by}
				costs={costs.data}
				isPending={costs.isPending}
				isError={costs.isError}
				onChange={setBy}
			/>
			<AgentSheet row={opened} onClose={() => setOpenId(undefined)} />
		</Page>
	)
}
