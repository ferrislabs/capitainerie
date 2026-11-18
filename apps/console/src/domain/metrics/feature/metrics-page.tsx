import type { UseQueryResult } from '@tanstack/react-query'
import { useNavigate, useSearch } from '@tanstack/react-router'
import { type ReactNode, useMemo, useRef } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { TabNav } from '#/components/layout/tab-nav'
import { Tile } from '#/components/layout/tile'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { Panel, PanelEmpty } from '#/domain/audit/ui/rows'
import { formatCost, formatTokens } from '#/domain/dashboard/format'
import { toDay } from '#/domain/metrics/calendar'
import {
	assignSlots,
	type DayPoint,
	deltaPercent,
	fillDays,
	previousWindow,
	topSlices,
} from '#/domain/metrics/charts'
import {
	type MetricsQuery,
	useAgentCostReport,
	useDayCosts,
	useProjectCosts,
	useTaskCosts,
	useUserCosts,
} from '#/domain/metrics/hooks/use-metrics'
import { type CustomRange, PERIODS, periodWindow } from '#/domain/metrics/model'
import {
	agentRows,
	type CostRow,
	projectRows,
	taskRows,
	userRows,
} from '#/domain/metrics/rows'
import {
	DEFAULT_PERIOD,
	type MetricsSearch,
	parseMetricsSearch,
} from '#/domain/metrics/search'
import { METRICS_TABS, type MetricsTab } from '#/domain/metrics/tabs'
import { CostTable } from '#/domain/metrics/ui/cost-table'
import { DailyChart } from '#/domain/metrics/ui/daily-chart'
import { MetricsFilters } from '#/domain/metrics/ui/metrics-filters'
import { RangePicker } from '#/domain/metrics/ui/range-picker'
import { ShareDonut } from '#/domain/metrics/ui/share-donut'
import { Sparkline } from '#/domain/metrics/ui/sparkline'
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
	points,
	before,
	previousCost,
	slots,
}: {
	query: UseQueryResult<T>
	subject: string
	toRows: (rows: T['rows']) => CostRow[]
	showRuns: boolean
	periodLabel: string
	points: ReadonlyArray<DayPoint> | undefined
	before: ReadonlyArray<DayPoint> | undefined
	previousCost: number | undefined
	slots: Map<string, number>
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
	const costRows = toRows(rows)
	const slices = topSlices(
		costRows.map((row) => ({
			key: row.key,
			label: row.label,
			value: row.figures.cost_micros,
		})),
	)
	assignSlots(
		slots,
		slices.filter((slice) => !slice.other).map((slice) => slice.key),
	)
	const change =
		previousCost === undefined
			? null
			: deltaPercent(total.cost_micros, previousCost)
	const variation =
		change === null ? null : (
			<span className="figures block">
				{change >= 0 ? '▲' : '▼'} {change >= 0 ? '+' : ''}
				{Math.round(change)} % vs période précédente
			</span>
		)
	const trend = (pick: (point: DayPoint) => number, label: string) =>
		points ? <Sparkline values={points.map(pick)} label={label} /> : null
	return (
		<>
			<div className="mb-3 grid grid-cols-2 gap-2.5 lg:grid-cols-3">
				<Tile
					label="Coût"
					value={formatCost(total.cost_micros)}
					hint={
						<>
							{periodLabel}
							{variation}
							{trend((p) => p.cost_micros, 'Coût par jour')}
						</>
					}
				/>
				<Tile
					label="Jetons en entrée"
					value={formatTokens(total.input_tokens)}
					hint={
						<>
							{periodLabel}
							{trend((p) => p.input_tokens, 'Jetons en entrée par jour')}
						</>
					}
				/>
				<Tile
					label="Jetons en sortie"
					value={formatTokens(total.output_tokens)}
					hint={
						<>
							{periodLabel}
							{trend((p) => p.output_tokens, 'Jetons en sortie par jour')}
						</>
					}
				/>
			</div>
			{rows.length === 0 ? (
				<Panel>
					<PanelEmpty title="Aucune dépense sur cette période">
						Change la période ou retire un filtre.
					</PanelEmpty>
				</Panel>
			) : (
				<>
					<div className="mb-3 grid gap-2.5 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
						{points ? <DailyChart points={points} previous={before} /> : null}
						<ShareDonut
							title={`Répartition par ${subject.toLowerCase()}`}
							slices={slices}
							total={total.cost_micros}
							slotOf={(key) => slots.get(key) ?? 0}
						/>
					</div>
					<CostTable subject={subject} rows={costRows} showRuns={showRuns} />
				</>
			)}
		</>
	)
}

interface TabProps {
	query: MetricsQuery
	periodLabel: string
	points: ReadonlyArray<DayPoint> | undefined
	before: ReadonlyArray<DayPoint> | undefined
	previousCost: number | undefined
	slots: Map<string, number>
}

function ProjectsTab({
	query,
	periodLabel,
	points,
	before,
	previousCost,
	slots,
}: TabProps) {
	return (
		<ReportView
			query={useProjectCosts(query)}
			subject="Projet"
			toRows={projectRows}
			showRuns
			periodLabel={periodLabel}
			points={points}
			before={before}
			previousCost={previousCost}
			slots={slots}
		/>
	)
}

function AgentsTab({
	query,
	periodLabel,
	points,
	before,
	previousCost,
	slots,
}: TabProps) {
	return (
		<ReportView
			query={useAgentCostReport(query)}
			subject="Agent"
			toRows={agentRows}
			showRuns
			periodLabel={periodLabel}
			points={points}
			before={before}
			previousCost={previousCost}
			slots={slots}
		/>
	)
}

function TasksTab({
	query,
	periodLabel,
	points,
	before,
	previousCost,
	slots,
}: TabProps) {
	return (
		<ReportView
			query={useTaskCosts(query)}
			subject="Tâche"
			toRows={taskRows}
			showRuns
			periodLabel={periodLabel}
			points={points}
			before={before}
			previousCost={previousCost}
			slots={slots}
		/>
	)
}

function UsersTab({
	query,
	periodLabel,
	points,
	before,
	previousCost,
	slots,
}: TabProps) {
	return (
		<ReportView
			query={useUserCosts(query)}
			subject="Utilisateur"
			toRows={userRows}
			showRuns={false}
			periodLabel={periodLabel}
			points={points}
			before={before}
			previousCost={previousCost}
			slots={slots}
		/>
	)
}

const TAB_VIEWS: Record<MetricsTab, (props: TabProps) => ReactNode> = {
	projets: ProjectsTab,
	agents: AgentsTab,
	taches: TasksTab,
	utilisateurs: UsersTab,
}

function MetricsBody({
	tab,
	query,
	periodLabel,
}: {
	tab: MetricsTab
	query: MetricsQuery
	periodLabel: string
}) {
	const days = useDayCosts(query)
	const beforeWindow = useMemo(
		() => previousWindow(query.window),
		[query.window],
	)
	const beforeDays = useDayCosts({ ...query, window: beforeWindow })
	const slots = useRef(new Map<string, number>())
	const points = useMemo(
		() => (days.data ? fillDays(days.data.rows, query.window) : undefined),
		[days.data, query.window],
	)
	const before = useMemo(
		() =>
			beforeDays.data
				? fillDays(beforeDays.data.rows, beforeWindow)
				: undefined,
		[beforeDays.data, beforeWindow],
	)
	const TabView = TAB_VIEWS[tab]
	return (
		<TabView
			query={query}
			periodLabel={periodLabel}
			points={points}
			before={before}
			previousCost={beforeDays.data?.total.cost_micros}
			slots={slots.current}
		/>
	)
}

export function MetricsPage({ tab }: Props) {
	const navigate = useNavigate()
	const search = parseMetricsSearch(useSearch({ strict: false }))
	const period = search.periode ?? DEFAULT_PERIOD
	const range = useMemo<CustomRange>(
		() => ({ from: search.du ?? '', to: search.au ?? '' }),
		[search.du, search.au],
	)
	const agentId = search.agent ?? ''
	const cardId = search.tache ?? ''
	const update = (patch: MetricsSearch) =>
		void navigate({
			to: '/metrics/{-$tab}',
			params: { tab },
			search: parseMetricsSearch({ ...search, ...patch }),
			replace: true,
		})
	const agents = useTeamAgents().data?.items ?? []
	const cards = useCards().data?.items ?? []
	const window = useMemo(
		() => periodWindow(period, new Date(), range),
		[period, range],
	)
	const today = toDay(new Date())
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
					link: {
						to: '/metrics/{-$tab}',
						params: { tab: item.slug },
						search: (previous: Record<string, unknown>) => previous,
					},
				}))}
			/>
			<div className="mb-2.5 flex flex-wrap gap-2">
				<RangePicker
					period={period}
					range={range}
					today={today}
					onChange={(next, picked) =>
						update({ periode: next, du: picked.from, au: picked.to })
					}
				/>
				<MetricsFilters
					agents={agents.map((agent) => ({ id: agent.id, name: agent.name }))}
					tasks={cards.map((card) => ({ id: card.id, name: card.title }))}
					agentId={agentId}
					cardId={cardId}
					onAgent={(id) => update({ agent: id })}
					onCard={(id) => update({ tache: id })}
				/>
			</div>
			<MetricsBody
				tab={tab}
				query={{ window, agentId, cardId }}
				periodLabel={periodLabel}
			/>
		</Page>
	)
}
