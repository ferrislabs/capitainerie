import { Link } from '@tanstack/react-router'
import { Play } from 'lucide-react'
import { Button } from '#/components/ui/button'
import {
	type Routine,
	relativeTime,
	triggersSummary,
} from '#/domain/routines/model'
import { AgentLabel } from '#/domain/routines/ui/agent-avatar'
import { RunBadge } from '#/domain/routines/ui/run-badge'
import { Sw, Tbl, Td, Th, Tr } from '#/domain/sources/ui/mockup'

interface Props {
	routines: ReadonlyArray<Routine>
	agentOf: (id: string) => { name: string; hue: number } | undefined
	now: number
	busy: boolean
	onLaunch: (routine: Routine) => void
	onToggle: (routine: Routine, enabled: boolean) => void
}

function When({ iso, now }: { iso: string; now: number }) {
	return (
		<time dateTime={iso} title={new Date(iso).toLocaleString('fr-FR')}>
			{relativeTime(iso, now)}
		</time>
	)
}

export function RoutinesTable({
	routines,
	agentOf,
	now,
	busy,
	onLaunch,
	onToggle,
}: Props) {
	return (
		<Tbl>
			<thead>
				<tr>
					<Th>Routine</Th>
					<Th>Agent</Th>
					<Th>Déclencheurs</Th>
					<Th>Prochain passage</Th>
					<Th>Dernier passage</Th>
					<Th>
						<abbr title="Tâches créées · ignorés · échecs">Passages</abbr>
					</Th>
					<Th>
						<span className="sr-only">Lancer</span>
					</Th>
					<Th>Actif</Th>
				</tr>
			</thead>
			<tbody>
				{routines.map((routine) => (
					<Tr key={routine.id}>
						<Td>
							<Link
								to="/routines/$routineId/{-$tab}"
								params={{ routineId: routine.id }}
								className="font-medium text-foreground"
							>
								{routine.name}
							</Link>
							<div className="font-mono text-[11.5px] text-muted-foreground">
								{routine.slug}
							</div>
						</Td>
						<Td>
							<AgentLabel agent={agentOf(routine.agent_id)} />
						</Td>
						<Td className="text-muted-foreground">
							{triggersSummary(routine)}
						</Td>
						<Td className="whitespace-nowrap text-muted-foreground">
							{routine.archived ? (
								'—'
							) : !routine.enabled ? (
								'En pause'
							) : routine.next_run_at ? (
								<When iso={routine.next_run_at} now={now} />
							) : (
								'—'
							)}
						</Td>
						<Td className="whitespace-nowrap">
							{routine.last_run ? (
								<>
									<span className="text-muted-foreground">
										<When
											iso={routine.last_run_at ?? routine.last_run.started_at}
											now={now}
										/>
									</span>{' '}
									<RunBadge run={routine.last_run} />
								</>
							) : (
								<span className="text-muted-foreground">Jamais</span>
							)}
						</Td>
						<Td className="whitespace-nowrap tabular-nums">
							<span
								title={`${routine.counts.created} tâches créées, ${routine.counts.skipped} ignorés, ${routine.counts.failed} échecs`}
							>
								{routine.counts.created}
								<span className="text-muted-foreground">
									{` · ${routine.counts.skipped} · `}
								</span>
								<span
									className={
										routine.counts.failed > 0 ? 'text-signal-danger' : undefined
									}
								>
									{routine.counts.failed}
								</span>
							</span>
						</Td>
						<Td className="text-right">
							{routine.archived ? null : (
								<Button
									type="button"
									size="sm"
									variant="outline"
									disabled={busy}
									aria-label={`Lancer ${routine.name}`}
									onClick={() => onLaunch(routine)}
								>
									<Play aria-hidden="true" />
									Lancer
								</Button>
							)}
						</Td>
						<Td>
							<Sw
								aria-label={`Activer ${routine.name}`}
								checked={routine.enabled}
								disabled={busy || routine.archived}
								onCheckedChange={(checked) => onToggle(routine, checked)}
							/>
						</Td>
					</Tr>
				))}
			</tbody>
		</Tbl>
	)
}
