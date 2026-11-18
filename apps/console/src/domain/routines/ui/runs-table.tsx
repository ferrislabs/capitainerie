import { Link } from '@tanstack/react-router'
import {
	ORIGIN_LABEL,
	type RoutineRun,
	relativeTime,
	runDuration,
} from '#/domain/routines/model'
import { RunBadge } from '#/domain/routines/ui/run-badge'
import { Tbl, Td, Th, Tr } from '#/domain/sources/ui/kit'
import { shortTaskId, taskPathId } from '#/domain/tasks/task-ids'

interface Props {
	runs: ReadonlyArray<RoutineRun>
	now: number
}

export function RunsTable({ runs, now }: Props) {
	return (
		<Tbl>
			<thead>
				<tr>
					<Th>Prévu</Th>
					<Th>Origine</Th>
					<Th>Statut</Th>
					<Th>Durée</Th>
					<Th>Tâche</Th>
					<Th>Détail</Th>
				</tr>
			</thead>
			<tbody>
				{runs.map((run) => (
					<Tr key={run.id}>
						<Td className="whitespace-nowrap text-muted-foreground">
							<time
								dateTime={run.scheduled_for}
								title={new Date(run.scheduled_for).toLocaleString('fr-FR')}
							>
								{relativeTime(run.scheduled_for, now)}
							</time>
						</Td>
						<Td>{ORIGIN_LABEL[run.origin]}</Td>
						<Td>
							<RunBadge run={run} />
						</Td>
						<Td className="whitespace-nowrap text-muted-foreground">
							{runDuration(run)}
						</Td>
						<Td>
							{run.card_id ? (
								<Link
									to="/tasks/$taskId"
									params={{ taskId: taskPathId(run.card_id) }}
									className="font-mono text-[12px] text-primary"
								>
									{shortTaskId(run.card_id)}
								</Link>
							) : (
								<span className="text-muted-foreground">—</span>
							)}
						</Td>
						<Td className="max-w-80 text-[12px] [overflow-wrap:anywhere] text-muted-foreground">
							{run.reason ?? ''}
						</Td>
					</Tr>
				))}
			</tbody>
		</Tbl>
	)
}
