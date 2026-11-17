import { Link } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { Tile } from '#/components/layout/tile'
import type { ActivityEvent, DailyRuns } from '#/domain/audit/mock/audit-data'
import { ActivityList } from '#/domain/audit/ui/activity-list'
import { RunsChart } from '#/domain/audit/ui/runs-chart'
import type { DashboardStats } from '#/domain/dashboard/stats'
import { taskPathId } from '#/domain/tasks/task-ids'

export interface LiveAgent {
	id: string
	name: string
	task: string | null
	taskId: string | null
}

interface Props {
	stats: DashboardStats
	liveAgents: ReadonlyArray<LiveAgent>
	days: ReadonlyArray<DailyRuns>
	events: ReadonlyArray<ActivityEvent>
	spend: string
	budget: string
}

const linkClass =
	'rounded-lg outline-none focus-visible:ring-2 focus-visible:ring-ring'

function Panel({ children }: { children: ReactNode }) {
	return <div className="flex flex-col gap-4">{children}</div>
}

export function DashboardView({
	stats,
	liveAgents,
	days,
	events,
	spend,
	budget,
}: Props) {
	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Tableau de bord"
				description="Vue d'ensemble de l'organisation"
			/>
			<div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
				<Link to="/agents" className={linkClass}>
					<Tile label="Agents en direct" value={stats.running} />
				</Link>
				<Link
					to="/tasks"
					search={{ vue: 'liste', q: '' }}
					className={linkClass}
				>
					<Tile label="Tâches ouvertes" value={stats.open} />
				</Link>
				<Link
					to="/tasks"
					search={{ vue: 'liste', q: '' }}
					className={linkClass}
				>
					<Tile label="Tâches bloquées" value={stats.blocked} />
				</Link>
				<Link to="/approvals" className={linkClass}>
					<Tile
						label="Approbations en attente"
						value={stats.pending}
						hint="En attente du board"
					/>
				</Link>
			</div>
			<div className="grid items-start gap-4 lg:grid-cols-2">
				<Panel>
					<SectionCard title="Agents en direct" flush>
						{liveAgents.length === 0 ? (
							<p className="px-3 py-2 text-muted-foreground">
								Aucun agent en cours d'exécution.
							</p>
						) : (
							<ul className="divide-y">
								{liveAgents.map((a) => (
									<li key={a.id} className="flex items-center gap-3 px-3 py-2">
										<Link
											to="/agents/$agentId/{-$tab}"
											params={{ agentId: a.id }}
											className="font-medium hover:underline"
										>
											{a.name}
										</Link>
										{a.taskId ? (
											<Link
												to="/tasks/$taskId"
												params={{ taskId: taskPathId(a.taskId) }}
												className="font-mono text-xs text-link hover:underline"
											>
												{a.taskId}
											</Link>
										) : null}
										<span className="min-w-0 flex-1 truncate text-muted-foreground">
											{a.task ?? 'Aucune tâche ouverte'}
										</span>
									</li>
								))}
							</ul>
						)}
					</SectionCard>
					<SectionCard
						title="Exécutions par jour"
						description="14 derniers jours"
					>
						<RunsChart days={days} />
					</SectionCard>
				</Panel>
				<Panel>
					<ActivityList events={events} />
					<SectionCard title="Coûts du mois">
						<div className="flex items-end justify-between gap-3">
							<div className="flex flex-col">
								<span className="figures text-xl font-semibold">{spend}</span>
								<span className="text-xs text-muted-foreground">{budget}</span>
							</div>
							<Link
								to="/audit/{-$tab}"
								params={{ tab: 'couts' }}
								className="text-sm hover:underline"
							>
								Voir les coûts
							</Link>
						</div>
					</SectionCard>
				</Panel>
			</div>
		</Page>
	)
}
