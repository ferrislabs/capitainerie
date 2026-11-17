import { Link, type LinkProps } from '@tanstack/react-router'
import {
	Bot,
	Clock,
	DollarSign,
	Plus,
	Shield,
	TriangleAlert,
} from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { Tile } from '#/components/layout/tile'
import { Button } from '#/components/ui/button'
import { rateBars, stackedBars } from '#/domain/dashboard/chart'
import {
	ATTENTION,
	type AttentionTarget,
	AXIS_DATES,
	LIVE_RUNS,
	OTHER_RUNS_LABEL,
	RECENT_ACTIVITY,
	RUNS_14,
	type Severity,
	SUCCESS_14,
	TASKS_14,
} from '#/domain/dashboard/mock/dashboard-data'
import { AgentFace, HumanAvatar } from '#/domain/dashboard/ui/avatars'
import { ChartPanel } from '#/domain/dashboard/ui/chart-panel'
import { StatusGlyph } from '#/domain/dashboard/ui/status-glyph'
import type { DemoAgent, DemoTask } from '#/domain/demo'
import { shortName } from '#/domain/demo'
import { taskPathId } from '#/domain/tasks/task-ids'
import { cn } from '#/lib/utils'

interface Props {
	agents: ReadonlyArray<DemoAgent>
	recentTasks: ReadonlyArray<DemoTask>
	pendingApprovals: number
	onNewTask: () => void
}

const TASKS_LINK = {
	to: '/tasks',
	search: { vue: 'liste', q: '' },
} as const satisfies LinkProps

const AUDIT_LINK = { to: '/audit/{-$tab}' } as const satisfies LinkProps

const TARGETS: Readonly<Record<AttentionTarget, LinkProps>> = {
	routines: { to: '/routines' },
	task: { to: '/tasks/$taskId', params: { taskId: 'nat-611' } },
	tasks: TASKS_LINK,
	approvals: { to: '/approvals' },
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
const TAG = 'font-mono text-[11px] font-medium whitespace-nowrap text-faint'
const WHEN = 'font-mono text-[11.5px] whitespace-nowrap text-faint'
const PANEL_LINK =
	'rounded-sm text-xs text-primary focus-visible:outline-2 focus-visible:outline-ring'

function agentOf(agents: ReadonlyArray<DemoAgent>, id: string) {
	return agents.find((a) => a.id === id)
}

export function DashboardView({
	agents,
	recentTasks,
	pendingApprovals,
	onNewTask,
}: Props) {
	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Tableau de bord"
				description="Natalia · 27 agents · actualisé il y a 12 s"
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
			<div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2 lg:grid-cols-4">
				{LIVE_RUNS.map((run, index) => {
					const agent = agentOf(agents, run.agentId)
					const link: LinkProps = run.taskId
						? {
								to: '/tasks/$taskId',
								params: { taskId: taskPathId(run.taskId) },
							}
						: { to: '/audit/{-$tab}', params: { tab: 'runs' } }
					return (
						<Link
							// biome-ignore lint/suspicious/noArrayIndexKey: les cartes se répètent par agent
							key={`${run.agentId}-${index}`}
							{...link}
							className="flex min-w-0 flex-col gap-[7px] rounded-lg border bg-card px-2.5 py-[9px] focus-visible:outline-2 focus-visible:outline-ring"
						>
							<span className="flex min-w-0 items-center gap-[7px]">
								<AgentFace hue={agent?.hue ?? 0} size={18} />
								<span className="truncate font-medium">{agent?.name}</span>
							</span>
							<span className="flex min-w-0 items-center gap-[7px] rounded-md border bg-background px-2 py-[5px] text-muted-foreground">
								{run.scheduled ? (
									<Clock aria-hidden="true" className="size-4 shrink-0" />
								) : (
									<StatusGlyph status="progress" />
								)}
								<span className="flex-1 truncate">{run.label}</span>
								{run.taskId ? <span className={TAG}>{run.taskId}</span> : null}
							</span>
							<span className="text-right text-[11.5px] text-faint">
								{run.footer}
							</span>
						</Link>
					)
				})}
			</div>
			<div className="mt-1.5 mb-3 text-right">
				<Link
					{...AUDIT_LINK}
					className="rounded-sm text-xs text-primary focus-visible:outline-2 focus-visible:outline-ring"
				>
					{OTHER_RUNS_LABEL}
				</Link>
			</div>
			<div className="grid grid-cols-2 gap-2.5 lg:grid-cols-5">
				<Tile
					label="Agents activés"
					value="27"
					hint="0 en cours, 0 en pause, 0 erreur"
					icon={Bot}
					link={{ to: '/agents' }}
				/>
				<Tile
					label="Tâches en cours"
					value="9"
					hint="122 ouvertes"
					icon={Clock}
					link={TASKS_LINK}
				/>
				<Tile
					label="Bloquées"
					value="65"
					hint="+8 depuis hier"
					icon={TriangleAlert}
					tone="warn"
					link={TASKS_LINK}
				/>
				<Tile
					label="Dépense du mois"
					value="0,00 $"
					hint="0 % du budget de 400 $"
					icon={DollarSign}
					link={{ to: '/audit/{-$tab}', params: { tab: 'couts' } }}
				/>
				<Tile
					label="Approbations"
					value={pendingApprovals}
					hint="En attente du board"
					icon={Shield}
					link={{ to: '/approvals' }}
				/>
			</div>
			<div className="mt-2.5 grid grid-cols-1 gap-2.5 lg:grid-cols-3">
				<ChartPanel
					title="Activité des exécutions"
					dates={AXIS_DATES}
					bars={stackedBars(RUNS_14)}
					legend={[
						{
							label: 'Réussies',
							fill: 'fill-signal-success',
							dot: 'bg-signal-success',
						},
						{
							label: 'Échecs',
							fill: 'fill-signal-danger',
							dot: 'bg-signal-danger',
						},
					]}
				/>
				<ChartPanel
					title="Tâches par statut"
					dates={AXIS_DATES}
					bars={stackedBars(TASKS_14)}
					legend={[
						{
							label: 'Terminé',
							fill: 'fill-signal-success',
							dot: 'bg-signal-success',
						},
						{ label: 'En cours', fill: 'fill-primary', dot: 'bg-primary' },
						{ label: 'En revue', fill: 'fill-violet', dot: 'bg-violet' },
						{
							label: 'À faire',
							fill: 'fill-signal-attention',
							dot: 'bg-signal-attention',
						},
						{
							label: 'Bloqué',
							fill: 'fill-signal-danger',
							dot: 'bg-signal-danger',
						},
					]}
				/>
				<ChartPanel
					title="Taux de réussite"
					dates={AXIS_DATES}
					bars={rateBars(SUCCESS_14)}
					legend={[
						{
							label: 'Réussite des exécutions',
							fill: 'fill-signal-success',
							dot: 'bg-signal-success',
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
							<span className="rounded-lg bg-secondary px-1.5 text-[11px] text-muted-foreground">
								{ATTENTION.length}
							</span>
							<Link {...TASKS_LINK} className={PANEL_LINK}>
								Tout voir
							</Link>
						</>
					}
				>
					<ul>
						{ATTENTION.map((item) => {
							const severity = SEVERITY[item.severity]
							return (
								<li key={item.title}>
									<Link {...TARGETS[item.target]} className={cn(ROW, HOVER)}>
										<span
											className={cn(
												'min-w-[62px] rounded-md border px-2 py-px text-center text-[11px] font-medium whitespace-nowrap',
												severity.tone,
											)}
										>
											{severity.label}
										</span>
										<span className="min-w-0 flex-1">
											<b className="block truncate font-medium">{item.title}</b>
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
				</SectionCard>
				<SectionCard
					title="Activité récente"
					flush
					className="mb-0"
					actions={
						<Link {...AUDIT_LINK} className={PANEL_LINK}>
							Journal
						</Link>
					}
				>
					<ul>
						{RECENT_ACTIVITY.map((event) => {
							const agent =
								event.kind === 'agent'
									? agents.find((a) => shortName(a.name) === event.who)
									: undefined
							return (
								<li
									key={`${event.who}-${event.what}`}
									className={cn(ROW, 'hover:bg-secondary')}
								>
									{event.kind === 'agent' ? (
										<AgentFace hue={agent?.hue ?? 0} />
									) : (
										<HumanAvatar initials={event.initials} />
									)}
									<span className="min-w-0 flex-1">
										<b className="block overflow-hidden font-medium text-ellipsis">
											{event.who}{' '}
											<span className="inline text-xs font-normal whitespace-nowrap text-muted-foreground">
												{event.what}
											</span>
										</b>
									</span>
									<span className={WHEN}>{event.when}</span>
								</li>
							)
						})}
					</ul>
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
				<ul>
					{recentTasks.map((task) => {
						const agent = agentOf(agents, task.agentId)
						return (
							<li key={task.id}>
								<Link
									to="/tasks/$taskId"
									params={{ taskId: taskPathId(task.id) }}
									className={cn(ROW, HOVER)}
								>
									<StatusGlyph status={task.status} />
									<span className={TAG}>{task.id}</span>
									<span className="min-w-0 flex-1">
										<b className="block truncate font-medium">{task.title}</b>
										{task.note ? (
											<span className="block truncate text-xs leading-normal text-muted-foreground">
												{task.note}
											</span>
										) : null}
									</span>
									<span className="flex min-w-0 items-center gap-1.5 overflow-hidden text-xs whitespace-nowrap text-muted-foreground">
										<AgentFace hue={agent?.hue ?? 0} size={18} />
										{agent ? shortName(agent.name) : null}
									</span>
									<span className={WHEN}>{task.age}</span>
								</Link>
							</li>
						)
					})}
				</ul>
			</SectionCard>
		</Page>
	)
}
