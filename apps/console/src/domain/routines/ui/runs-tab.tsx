import { Link } from '@tanstack/react-router'
import { Play } from 'lucide-react'
import { EmptyState } from '#/components/layout/empty-state'
import { SectionCard } from '#/components/layout/section-card'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import type { RoutineRun } from '#/domain/routines/types'
import { RunBadge } from '#/domain/routines/ui/run-badge'
import { taskPathId } from '#/domain/tasks/task-ids'

interface Props {
	runs: ReadonlyArray<RoutineRun>
	knownTaskIds: ReadonlySet<string>
}

export function RunsTab({ runs, knownTaskIds }: Props) {
	return (
		<SectionCard title="Exécutions" bare>
			{runs.length === 0 ? (
				<EmptyState
					icon={Play}
					title="Jamais lancée"
					description="Utilise « Lancer maintenant » pour créer une première tâche."
				/>
			) : (
				<Table>
					<TableHeader>
						<TableRow>
							<TableHead>Quand</TableHead>
							<TableHead>Déclencheur</TableHead>
							<TableHead>Statut</TableHead>
							<TableHead>Durée</TableHead>
							<TableHead>Tâche</TableHead>
						</TableRow>
					</TableHeader>
					<TableBody>
						{runs.map((run) => (
							<TableRow key={run.id}>
								<TableCell className="whitespace-nowrap text-muted-foreground">
									{run.when}
								</TableCell>
								<TableCell>{run.origin}</TableCell>
								<TableCell>
									<RunBadge tone={run.tone} label={run.label} />
								</TableCell>
								<TableCell className="text-muted-foreground">
									{run.duration}
								</TableCell>
								<TableCell className="font-mono">
									{knownTaskIds.has(run.taskId) ? (
										<Link
											to="/tasks/$taskId"
											params={{ taskId: taskPathId(run.taskId) }}
											className="text-link hover:underline"
										>
											{run.taskId}
										</Link>
									) : (
										run.taskId
									)}
								</TableCell>
							</TableRow>
						))}
					</TableBody>
				</Table>
			)}
		</SectionCard>
	)
}
