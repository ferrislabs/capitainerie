import type { UseQueryResult } from '@tanstack/react-query'
import { Link, type LinkProps } from '@tanstack/react-router'
import {
	Bot,
	Clock,
	DollarSign,
	ListChecks,
	Plus,
	TriangleAlert,
} from 'lucide-react'
import type { ReactNode } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { Tile } from '#/components/layout/tile'
import { Button } from '#/components/ui/button'
import { axisDates, dailySeries, stackedBars } from '#/domain/dashboard/chart'
import {
	formatCost,
	formatCount,
	formatTokens,
	hueOf,
	relativeAge,
} from '#/domain/dashboard/format'
import {
	type AttentionTarget,
	activeAgentCount,
	attentionItems,
	cardStats,
	recentCards,
	type Severity,
} from '#/domain/dashboard/stats'
import { AgentFace } from '#/domain/dashboard/ui/avatars'
import { ChartPanel, type PanelStatus } from '#/domain/dashboard/ui/chart-panel'
import { StatusGlyph } from '#/domain/dashboard/ui/status-glyph'
import { statusOfColumn } from '#/domain/tasks/model'
import { cn } from '#/lib/utils'

interface Props {
	active: UseQueryResult<Schemas.ActiveRunListResponse>
	runs: UseQueryResult<Schemas.RunListResponse>
	costs: UseQueryResult<Schemas.CostsResponse>
	days: ReadonlyArray<string>
	cards: UseQueryResult<Schemas.CardListResponse>
	agents: UseQueryResult<Schemas.AgentResponse[]>
	onNewTask: () => void
}

const LIVE_CARDS = 4
const RECENT_TASKS = 7

const TASKS_LINK = {
	to: '/tasks',
	search: { vue: 'liste', q: '' },
} as const satisfies LinkProps

const AUDIT_LINK = { to: '/audit/{-$tab}' } as const satisfies LinkProps
const RUNS_LINK = {
	to: '/audit/{-$tab}',
	params: { tab: 'runs' },
} as const satisfies LinkProps

const TARGETS: Readonly<Record<AttentionTarget, LinkProps>> = {
	runs: RUNS_LINK,
	tasks: TASKS_LINK,
}

const SEVERITY: Readonly<Record<Severity, { label: string; tone: string }>> = {
	crit: {
		label: 'Critique',
		tone: 'border-signal-danger/45 bg-signal-danger/12 text-signal-danger',
	},
	warn: {
		label: 'Attention',
		tone: 'border-signal-attention-ink/45 bg-signal-attention-ink/12 text-signal-attention-ink',
	},
	info: {
		label: 'Info',
		tone: 'border-border bg-secondary text-muted-foreground',
	},
}

const ROW =
	'flex min-h-9 w-full min-w-0 items-center gap-2.5 border-t px-3 py-1.5 text-left first:border-t-0'
const HOVER =
	'hover:bg-secondary focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring'
const EYEBROW =
	'mb-2 font-mono text-[10.5px] leading-none font-medium tracking-[0.09em] text-faint uppercase'
const WHEN = 'font-mono text-[11.5px] whitespace-nowrap text-faint'
const PANEL_LINK =
	'rounded-sm text-xs text-primary focus-visible:outline-2 focus-visible:outline-ring'

const LOADING = '…'
const UNAVAILABLE = '–'

function statusOf(query: {
	isPending: boolean
	isError: boolean
}): PanelStatus {
	if (query.isError) return 'error'
	return query.isPending ? 'loading' : 'ready'
}

function figure<T>(
	query: UseQueryResult<T>,
	read: (data: T) => ReactNode,
): ReactNode {
	if (query.isError) return UNAVAILABLE
	return query.data === undefined ? LOADING : read(query.data)
}

function PanelMessage({
	query,
	empty,
}: {
	query: { isPending: boolean; isError: boolean }
	empty: string
}) {
	if (query.isError) {
		return (
			<div className="p-3">
				<InlineAlert tone="danger">Données indisponibles.</InlineAlert>
			</div>
		)
	}
	return (
		<output className="block px-3 py-4 text-[13px] text-muted-foreground">
			{query.isPending ? 'Chargement…' : empty}
		</output>
	)
}

function liveLabel(item: Schemas.ActiveRunResponse): string {
	const { run } = item
	return (
		run.title ?? run.transcript_title ?? run.repository ?? 'Exécution en cours'
	)
}

function liveFooter(item: Schemas.ActiveRunResponse): string {
	const { run } = item
	if (run.status === 'queued') {
		return `En file ${relativeAge(run.created_at).replace('il y a ', 'depuis ')}`.trim()
	}
	const step = `Étape ${item.steps_done}/${item.steps_total}`
	return run.started_at ? `${step} · ${relativeAge(run.started_at)}` : step
}

function runEvent(run: Schemas.RunResponse): string {
	const title = run.title ?? run.transcript_title ?? 'Exécution'
	const verb: Readonly<Record<Schemas.RunStatusResponse, string>> = {
		queued: 'en file',
		running: 'en cours',
		succeeded: 'terminée',
		failed: 'en échec',
		cancelled: 'annulée',
	}
	return `${title} · ${verb[run.status]}`
}

export function DashboardView({
	active,
	runs,
	costs,
	days,
	cards,
	agents,
	onNewTask,
}: Props) {
	const activeItems = active.data?.items ?? []
	const cardItems = cards.data?.items ?? []
	const runItems = runs.data?.items ?? []
	const stats = cardStats(cardItems)
	const dates = axisDates(days)
	const costRows = costs.data?.rows ?? []
	const costStatus = statusOf(costs)
	const attention = attentionItems(cardItems, runItems)
	const queued = activeItems.filter(
		(item) => item.run.status === 'queued',
	).length
	const extraActive = Math.max(0, activeItems.length - LIVE_CARDS)
	const description = agents.data
		? `${formatCount(agents.data.length)} ${agents.data.length > 1 ? 'agents' : 'agent'} · ${formatCount(activeItems.length)} ${activeItems.length > 1 ? 'exécutions actives' : 'exécution active'}`
		: undefined

	return (
		<Page>
			<PageHeader
				title="Tableau de bord"
				description={description}
				actions={
					<>
						<Button asChild variant="outline">
							<Link {...AUDIT_LINK}>Journal d'audit</Link>
						</Button>
						<Button onClick={onNewTask}>
							<Plus aria-hidden="true" />
							Nouvelle tâche
						</Button>
					</>
				}
			/>
			<h2 className={EYEBROW}>Agents en direct</h2>
			{active.isError ? (
				<InlineAlert tone="danger">
					Impossible de charger les exécutions en cours.
				</InlineAlert>
			) : active.isPending ? (
				<output className="block text-[13px] text-muted-foreground">
					Chargement…
				</output>
			) : activeItems.length === 0 ? (
				<p className="rounded-lg border border-dashed bg-card px-3 py-4 text-[13px] text-muted-foreground">
					Aucun agent en cours d'exécution.
				</p>
			) : (
				<div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2 lg:grid-cols-4">
					{activeItems.slice(0, LIVE_CARDS).map((item) => {
						const { run } = item
						const name = item.current_step?.agent ?? 'Agent'
						const link: LinkProps = run.card_id
							? { to: '/tasks/$taskId', params: { taskId: run.card_id } }
							: RUNS_LINK
						return (
							<Link
								key={run.id}
								{...link}
								className="flex min-w-0 flex-col gap-[7px] rounded-lg border bg-card px-2.5 py-[9px] focus-visible:outline-2 focus-visible:outline-ring"
							>
								<span className="flex min-w-0 items-center gap-[7px]">
									<AgentFace hue={hueOf(name)} size={18} />
									<span className="truncate font-medium">{name}</span>
								</span>
								<span className="flex min-w-0 items-center gap-[7px] rounded-md border bg-background px-2 py-[5px] text-muted-foreground">
									{run.status === 'queued' ? (
										<Clock aria-hidden="true" className="size-4 shrink-0" />
									) : (
										<StatusGlyph status="progress" />
									)}
									<span className="flex-1 truncate">{liveLabel(item)}</span>
								</span>
								<span className="text-right text-[11.5px] text-faint">
									{liveFooter(item)}
								</span>
							</Link>
						)
					})}
				</div>
			)}
			<div className="mt-1.5 mb-3 text-right">
				<Link {...RUNS_LINK} className={PANEL_LINK}>
					{extraActive > 0
						? `${formatCount(extraActive)} ${extraActive > 1 ? 'autres exécutions actives' : 'autre exécution active'}`
						: runs.data
							? `${formatCount(runs.data.total)} ${runs.data.total > 1 ? 'exécutions au total' : 'exécution au total'}`
							: 'Toutes les exécutions'}
				</Link>
			</div>
			<div className="grid grid-cols-2 gap-2.5 lg:grid-cols-5">
				<Tile
					label="Agents"
					value={figure(agents, (list) => formatCount(list.length))}
					hint={
						active.data
							? `${formatCount(activeAgentCount(activeItems))} en cours`
							: undefined
					}
					icon={Bot}
					link={{ to: '/agents' }}
				/>
				<Tile
					label="Tâches en cours"
					value={figure(cards, () => formatCount(stats.inProgress))}
					hint={cards.data ? `${formatCount(stats.open)} ouvertes` : undefined}
					icon={Clock}
					link={TASKS_LINK}
				/>
				<Tile
					label="À valider"
					value={figure(cards, () => formatCount(stats.proposed))}
					hint="Tâches proposées"
					icon={TriangleAlert}
					tone={stats.proposed > 0 ? 'warn' : 'default'}
					link={TASKS_LINK}
				/>
				<Tile
					label="Dépense sur 14 jours"
					value={figure(costs, (data) => formatCost(data.total.cost_micros))}
					hint={
						costs.data
							? `${formatTokens(costs.data.total.input_tokens + costs.data.total.output_tokens)} jetons`
							: undefined
					}
					icon={DollarSign}
					link={{ to: '/audit/{-$tab}', params: { tab: 'couts' } }}
				/>
				<Tile
					label="Exécutions actives"
					value={figure(active, (data) => formatCount(data.items.length))}
					hint={active.data ? `${formatCount(queued)} en file` : undefined}
					icon={ListChecks}
					link={RUNS_LINK}
				/>
			</div>
			<div className="mt-2.5 grid grid-cols-1 gap-2.5 lg:grid-cols-3">
				<ChartPanel
					title="Activité des exécutions"
					status={costStatus}
					dates={dates}
					bars={stackedBars(
						dailySeries(costRows, days, 1, (row) => [row.runs]),
					)}
					legend={[
						{
							label: 'Exécutions',
							fill: 'fill-signal-success',
							dot: 'bg-signal-success',
						},
					]}
				/>
				<ChartPanel
					title="Jetons par jour"
					status={costStatus}
					dates={dates}
					bars={stackedBars(
						dailySeries(costRows, days, 2, (row) => [
							row.input_tokens,
							row.output_tokens,
						]),
					)}
					legend={[
						{ label: 'Entrée', fill: 'fill-primary', dot: 'bg-primary' },
						{ label: 'Sortie', fill: 'fill-violet', dot: 'bg-violet' },
					]}
				/>
				<ChartPanel
					title="Coût par jour"
					status={costStatus}
					dates={dates}
					bars={stackedBars(
						dailySeries(costRows, days, 1, (row) => [row.cost_micros]),
					)}
					legend={[
						{
							label: 'Dépense en dollars',
							fill: 'fill-signal-attention',
							dot: 'bg-signal-attention',
						},
					]}
				/>
			</div>
			<div className="mt-2.5 grid grid-cols-1 items-start gap-2.5 lg:grid-cols-[minmax(0,1.6fr)_minmax(0,1fr)]">
				<SectionCard
					title="À traiter"
					flush
					className="mb-0"
					actions={
						<>
							{cards.data && runs.data ? (
								<span className="rounded-lg bg-secondary px-1.5 text-[11px] text-muted-foreground">
									{attention.length}
								</span>
							) : null}
							<Link {...TASKS_LINK} className={PANEL_LINK}>
								Tout voir
							</Link>
						</>
					}
				>
					{cards.data && runs.data ? (
						attention.length === 0 ? (
							<PanelMessage
								query={{ isPending: false, isError: false }}
								empty="Rien à traiter pour le moment."
							/>
						) : (
							<ul>
								{attention.map((item) => {
									const severity = SEVERITY[item.severity]
									return (
										<li key={item.title}>
											<Link
												{...TARGETS[item.target]}
												className={cn(ROW, HOVER)}
											>
												<span
													className={cn(
														'min-w-[62px] rounded-md border px-2 py-px text-center text-[11px] font-medium whitespace-nowrap',
														severity.tone,
													)}
												>
													{severity.label}
												</span>
												<span className="min-w-0 flex-1">
													<b className="block truncate font-medium">
														{item.title}
													</b>
													<span className="block truncate text-xs leading-normal text-muted-foreground">
														{item.detail}
													</span>
												</span>
												<span className="inline-flex h-6 items-center rounded-md border bg-card px-2 text-xs font-medium whitespace-nowrap">
													{item.action}
												</span>
											</Link>
										</li>
									)
								})}
							</ul>
						)
					) : (
						<PanelMessage
							query={{
								isPending: !(cards.isError || runs.isError),
								isError: cards.isError || runs.isError,
							}}
							empty=""
						/>
					)}
				</SectionCard>
				<SectionCard
					title="Activité récente"
					flush
					className="mb-0"
					actions={
						<Link {...RUNS_LINK} className={PANEL_LINK}>
							Journal
						</Link>
					}
				>
					{runs.data && runItems.length > 0 ? (
						<ul>
							{runItems.slice(0, RECENT_TASKS).map((run) => (
								<li key={run.id} className={cn(ROW, 'hover:bg-secondary')}>
									<AgentFace hue={hueOf(run.initiated_by)} />
									<span className="min-w-0 flex-1">
										<b className="block truncate font-medium">
											{runEvent(run)}
										</b>
									</span>
									<span className={WHEN}>
										{relativeAge(run.finished_at ?? run.created_at)}
									</span>
								</li>
							))}
						</ul>
					) : (
						<PanelMessage
							query={runs}
							empty="Aucune exécution pour le moment."
						/>
					)}
				</SectionCard>
			</div>
			<SectionCard
				title="Tâches récentes"
				flush
				className="mt-2.5"
				actions={
					<Link {...TASKS_LINK} className={PANEL_LINK}>
						Toutes les tâches
					</Link>
				}
			>
				{cards.data && recentCards(cardItems, RECENT_TASKS).length > 0 ? (
					<ul>
						{recentCards(cardItems, RECENT_TASKS).map((card) => (
							<li key={card.id}>
								<Link
									to="/tasks/$taskId"
									params={{ taskId: card.id }}
									className={cn(ROW, HOVER)}
								>
									<StatusGlyph status={statusOfColumn(card.column)} />
									<span className="min-w-0 flex-1">
										<b className="block truncate font-medium">{card.title}</b>
									</span>
									{card.project ? (
										<span className="min-w-0 truncate text-xs whitespace-nowrap text-muted-foreground">
											{card.project.name}
										</span>
									) : null}
									<span className={WHEN}>{relativeAge(card.updated_at)}</span>
								</Link>
							</li>
						))}
					</ul>
				) : (
					<PanelMessage query={cards} empty="Aucune tâche pour le moment." />
				)}
			</SectionCard>
		</Page>
	)
}
