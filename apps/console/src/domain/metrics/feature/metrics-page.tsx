import type { UseQueryResult } from '@tanstack/react-query'
import { type ReactNode, useMemo, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { TabNav } from '#/components/layout/tab-nav'
import { Tile } from '#/components/layout/tile'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { Panel, PanelEmpty } from '#/domain/audit/ui/rows'
import { formatCost, formatTokens } from '#/domain/dashboard/format'
import {
	type MetricsQuery,
	useAgentCostReport,
	useProjectCosts,
	useTaskCosts,
	useUserCosts,
} from '#/domain/metrics/hooks/use-metrics'
import {
	type CustomRange,
	isValidWindow,
	PERIODS,
	type Period,
	periodWindow,
} from '#/domain/metrics/model'
import {
	agentRows,
	type CostRow,
	projectRows,
	taskRows,
	userRows,
} from '#/domain/metrics/rows'
import { METRICS_TABS, type MetricsTab } from '#/domain/metrics/tabs'
import { CostTable } from '#/domain/metrics/ui/cost-table'
import { MetricsFilters } from '#/domain/metrics/ui/metrics-filters'
import { PeriodPicker } from '#/domain/metrics/ui/period-picker'
import { useCards } from '#/domain/tasks/hooks/use-tasks'

interface Props {
	tab: MetricsTab
}

interface Report {
	total: { input_tokens: number; output_tokens: number; cost_micros: number }
	rows: ReadonlyArray<unknown>
}

function failureMessage(error: unknown): string {
	const status = (error as { status?: number } | null)?.status
	if (status === 403)
		return "Tu n'as pas accès aux métriques de l'organisation."
	if (status === 404) return "L'agent ou la tâche choisi n'existe plus."
	return 'Impossible de charger les métriques.'
}

function ReportView<T extends Report>({
	query,
	subject,
	toRows,
	showRuns,
	periodLabel,
}: {
	query: UseQueryResult<T>
	subject: string
	toRows: (rows: T['rows']) => CostRow[]
	showRuns: boolean
	periodLabel: string
}) {
	if (query.isPending)
		return (
			<output className="block text-[13px] text-muted-foreground">
				Chargement…
			</output>
		)
	if (query.isError)
		return (
			<InlineAlert tone="danger">{failureMessage(query.error)}</InlineAlert>
		)
	const { total, rows } = query.data
	return (
		<>
			<div className="mb-3 grid grid-cols-2 gap-2.5 lg:grid-cols-3">
				<Tile
					label="Coût"
					value={formatCost(total.cost_micros)}
					hint={periodLabel}
				/>
				<Tile
					label="Jetons en entrée"
					value={formatTokens(total.input_tokens)}
					hint={periodLabel}
				/>
				<Tile
					label="Jetons en sortie"
					value={formatTokens(total.output_tokens)}
					hint={periodLabel}
				/>
			</div>
			{rows.length === 0 ? (
				<Panel>
					<PanelEmpty title="Aucune dépense sur cette période">
						Change la période ou retire un filtre.
					</PanelEmpty>
				</Panel>
			) : (
				<CostTable subject={subject} rows={toRows(rows)} showRuns={showRuns} />
			)}
		</>
	)
}

interface TabProps {
	query: MetricsQuery
	periodLabel: string
}

function ProjectsTab({ query, periodLabel }: TabProps) {
	return (
		<ReportView
			query={useProjectCosts(query)}
			subject="Projet"
			toRows={projectRows}
			showRuns
			periodLabel={periodLabel}
		/>
	)
}

function AgentsTab({ query, periodLabel }: TabProps) {
	return (
		<ReportView
			query={useAgentCostReport(query)}
			subject="Agent"
			toRows={agentRows}
			showRuns
			periodLabel={periodLabel}
		/>
	)
}

function TasksTab({ query, periodLabel }: TabProps) {
	return (
		<ReportView
			query={useTaskCosts(query)}
			subject="Tâche"
			toRows={taskRows}
			showRuns
			periodLabel={periodLabel}
		/>
	)
}

function UsersTab({ query, periodLabel }: TabProps) {
	return (
		<ReportView
			query={useUserCosts(query)}
			subject="Utilisateur"
			toRows={userRows}
			showRuns={false}
			periodLabel={periodLabel}
		/>
	)
}

const TAB_VIEWS: Record<MetricsTab, (props: TabProps) => ReactNode> = {
	projets: ProjectsTab,
	agents: AgentsTab,
	taches: TasksTab,
	utilisateurs: UsersTab,
}

export function MetricsPage({ tab }: Props) {
	const [period, setPeriod] = useState<Period>('30j')
	const [range, setRange] = useState<CustomRange>({ from: '', to: '' })
	const [agentId, setAgentId] = useState('')
	const [cardId, setCardId] = useState('')
	const agents = useTeamAgents().data?.items ?? []
	const cards = useCards().data?.items ?? []
	const window = useMemo(
		() => periodWindow(period, new Date(), range),
		[period, range],
	)
	const TabView = TAB_VIEWS[tab]
	const valid = window !== null && isValidWindow(window)
	const periodLabel =
		PERIODS.find((item) => item.value === period)?.label.toLowerCase() ?? ''
	return (
		<Page>
			<PageHeader
				title="Métriques"
				description="Coûts de l'organisation par projet, agent, tâche et utilisateur"
			/>
			<TabNav
				label="Sections des métriques"
				tabs={METRICS_TABS.map((item) => ({
					label: item.label,
					link: { to: '/metrics/{-$tab}', params: { tab: item.slug } },
				}))}
			/>
			<div className="mb-2.5 flex flex-wrap gap-2">
				<PeriodPicker
					period={period}
					range={range}
					invalid={window !== null && !valid}
					onPeriod={setPeriod}
					onRange={setRange}
				/>
				<MetricsFilters
					agents={agents.map((agent) => ({ id: agent.id, name: agent.name }))}
					tasks={cards.map((card) => ({ id: card.id, name: card.title }))}
					agentId={agentId}
					cardId={cardId}
					onAgent={setAgentId}
					onCard={setCardId}
				/>
			</div>
			{window === null ? (
				<InlineAlert tone="info">Choisis les deux dates.</InlineAlert>
			) : !valid ? (
				<InlineAlert tone="danger">
					La date de début doit précéder la date de fin.
				</InlineAlert>
			) : (
				<TabView
					query={{ window, agentId, cardId }}
					periodLabel={periodLabel}
				/>
			)}
		</Page>
	)
}
