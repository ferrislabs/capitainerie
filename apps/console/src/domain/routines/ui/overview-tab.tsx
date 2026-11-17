import { Link } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Tile } from '#/components/layout/tile'
import { DEMO_AGENTS } from '#/domain/demo'
import { formatRun } from '#/domain/routines/schedule'
import type { Routine } from '#/domain/routines/types'
import { NextRuns } from '#/domain/routines/ui/next-runs'
import { RunBadge } from '#/domain/routines/ui/run-badge'

interface Props {
	routine: Routine
	scheduleLabel: string
	upcoming: ReadonlyArray<Date>
}

function Row({ label, children }: { label: string; children: ReactNode }) {
	return (
		<div className="flex items-baseline justify-between gap-4 border-b py-1.5 text-[13px] last:border-b-0">
			<dt className="text-muted-foreground">{label}</dt>
			<dd className="min-w-0 text-right">{children}</dd>
		</div>
	)
}

export function OverviewTab({ routine, scheduleLabel, upcoming }: Props) {
	const total = routine.runs.length
	const succeeded = routine.runs.filter((r) => r.tone !== 'danger').length
	const first = upcoming[0]
	return (
		<div className="flex flex-col gap-4">
			<div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
				<Tile
					label="Prochain passage"
					value={first ? formatRun(first) : routine.enabled ? '—' : 'En pause'}
					hint={scheduleLabel}
				/>
				<Tile
					label="Dernier passage"
					value={routine.lastRun.when}
					hint={
						<RunBadge
							tone={routine.lastRun.tone}
							label={routine.lastRun.label}
						/>
					}
				/>
				<Tile
					label="Réussite"
					value={total > 0 ? `${succeeded} / ${total}` : '—'}
					hint="derniers passages"
				/>
				<Tile label="Tâches créées" value={total} hint="au total" />
			</div>
			<div className="grid items-start gap-4 lg:grid-cols-2">
				<SectionCard title="Consigne">
					<p className="text-[13px] whitespace-pre-wrap">
						{routine.instruction}
					</p>
				</SectionCard>
				<SectionCard title="Configuration">
					<dl>
						<Row label="Agent">
							{DEMO_AGENTS.find((a) => a.name === routine.agent) ? (
								<Link
									to="/agents/equipe/$agentId/{-$tab}"
									params={{
										agentId:
											DEMO_AGENTS.find((a) => a.name === routine.agent)?.id ??
											'',
									}}
									className="hover:underline"
								>
									{routine.agent}
								</Link>
							) : (
								routine.agent
							)}
						</Row>
						<Row label="Projet">{routine.project}</Row>
						<Row label="Priorité">{routine.priority}</Row>
						<Row label="Titre de la tâche">
							<span className="font-mono">{routine.titleTemplate}</span>
						</Row>
						<Row label="Si déjà ouverte">{routine.ifOpen}</Row>
						<Row label="Si manqué">{routine.ifMissed}</Row>
						<Row label="Fuseau">{routine.timezone}</Row>
					</dl>
				</SectionCard>
			</div>
			<SectionCard title="Prochains passages">
				<NextRuns
					runs={upcoming}
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
