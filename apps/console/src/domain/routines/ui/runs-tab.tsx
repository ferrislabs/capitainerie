import { Link } from '@tanstack/react-router'
import { SectionCard } from '#/components/layout/section-card'
import type { RoutineRun } from '#/domain/routines/types'
import { RunBadge } from '#/domain/routines/ui/run-badge'
import { Empty, Tbl, Td, Th, Tr } from '#/domain/sources/ui/mockup'
import { taskPathId } from '#/domain/tasks/task-ids'

interface Props {
	runs: ReadonlyArray<RoutineRun>
	knownTaskIds: ReadonlySet<string>
}

export function RunsTab({ runs, knownTaskIds }: Props) {
	return (
		<SectionCard title="Exécutions" flush>
			{runs.length === 0 ? (
				<Empty title="Jamais lancée">
					Utilise « Lancer maintenant » pour créer une première tâche.
				</Empty>
			) : (
				<Tbl>
					<thead>
						<tr>
							<Th>Quand</Th>
							<Th>Déclencheur</Th>
							<Th>Statut</Th>
							<Th>Durée</Th>
							<Th>Tâche</Th>
						</tr>
					</thead>
					<tbody>
						{runs.map((run) => (
							<Tr key={run.id}>
								<Td className="whitespace-nowrap text-muted-foreground">
									{run.when}
								</Td>
								<Td>{run.origin}</Td>
								<Td>
									<RunBadge tone={run.tone} label={run.label} />
								</Td>
								<Td className="text-muted-foreground">{run.duration}</Td>
								<Td>
									{knownTaskIds.has(run.taskId) ? (
										<Link
											to="/tasks/$taskId"
											params={{ taskId: taskPathId(run.taskId) }}
											className="text-[12px] text-primary"
										>
											{run.taskId}
										</Link>
									) : (
										<span className="text-[12px] text-primary">
											{run.taskId}
										</span>
									)}
								</Td>
							</Tr>
						))}
					</tbody>
				</Tbl>
			)}
		</SectionCard>
	)
}
