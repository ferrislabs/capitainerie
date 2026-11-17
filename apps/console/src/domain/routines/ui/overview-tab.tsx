import { Link } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { DEMO_AGENTS } from '#/domain/demo'
import { formatRun } from '#/domain/routines/schedule'
import type { Routine } from '#/domain/routines/types'
import { NextRuns } from '#/domain/routines/ui/next-runs'
import { RunBadge } from '#/domain/routines/ui/run-badge'
import { Kv } from '#/domain/sources/ui/mockup'
import { cn } from '#/lib/utils'

interface Props {
	routine: Routine
	scheduleLabel: string
	upcoming: ReadonlyArray<Date>
}

function RoutineTile({
	label,
	value,
	hint,
	small = false,
}: {
	label: string
	value: ReactNode
	hint?: ReactNode
	small?: boolean
}) {
	return (
		<div className="flex min-w-0 flex-col gap-0.5 rounded-lg border bg-card px-3 pt-[11px] pb-2.5 text-left">
			<span className="flex items-center gap-1.5 text-[12px] font-semibold text-muted-foreground">
				{label}
			</span>
			<span
				className={cn(
					'figures [line-height:1.15] font-semibold tracking-[-0.02em]',
					small ? 'text-[15px]' : 'text-[30px]',
				)}
			>
				{value}
			</span>
			<span className="text-[11.5px] text-muted-foreground">{hint}</span>
		</div>
	)
}

export function OverviewTab({ routine, scheduleLabel, upcoming }: Props) {
	const total = routine.runs.length
	const succeeded = routine.runs.filter((r) => r.tone !== 'danger').length
	const first = upcoming[0]
	const agent = DEMO_AGENTS.find((a) => a.name === routine.agent)
	return (
		<div>
			<div className="mb-2.5 grid grid-cols-4 gap-2.5 max-[1100px]:grid-cols-2">
				<RoutineTile
					small
					label="Prochain passage"
					value={first ? formatRun(first) : routine.enabled ? '—' : 'En pause'}
					hint={scheduleLabel}
				/>
				<RoutineTile
					small
					label="Dernier passage"
					value={routine.lastRun.when}
					hint={
						<RunBadge
							tone={routine.lastRun.tone}
							label={routine.lastRun.label}
						/>
					}
				/>
				<RoutineTile
					label="Réussite"
					value={total > 0 ? `${succeeded} / ${total}` : '—'}
					hint="derniers passages"
				/>
				<RoutineTile label="Tâches créées" value={total} hint="au total" />
			</div>
			<div className="grid grid-cols-2 items-start gap-2.5 max-[1100px]:grid-cols-1">
				<SectionCard title="Consigne" flush>
					<p className="m-0 p-3 whitespace-pre-wrap">{routine.instruction}</p>
				</SectionCard>
				<SectionCard title="Configuration" flush>
					<Kv label="Agent">
						{agent ? (
							<Link
								to="/agents/$agentId/{-$tab}"
								params={{ agentId: agent.id }}
								className="text-foreground"
							>
								{routine.agent}
							</Link>
						) : (
							routine.agent
						)}
					</Kv>
					<Kv label="Projet">{routine.project}</Kv>
					<Kv label="Priorité">{routine.priority}</Kv>
					<Kv label="Titre de la tâche">
						<span className="font-mono text-[11.5px]">
							{routine.titleTemplate}
						</span>
					</Kv>
					<Kv label="Si déjà ouverte">{routine.ifOpen}</Kv>
					<Kv label="Si manqué">{routine.ifMissed}</Kv>
					<Kv label="Fuseau">{routine.timezone}</Kv>
				</SectionCard>
			</div>
			<SectionCard title="Prochains passages" flush>
				<NextRuns
					runs={upcoming}
					emptyClassName="py-3"
					empty={
						routine.enabled
							? 'Aucun passage calculable pour ce déclencheur.'
							: 'La routine est en pause : aucun passage prévu.'
					}
				/>
			</SectionCard>
		</div>
	)
}
