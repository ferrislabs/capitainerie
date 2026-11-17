import { Link } from '@tanstack/react-router'
import { Badge } from '#/components/ui/badge'
import {
	Table,
	TableBody,
	TableCell,
	TableHead,
	TableHeader,
	TableRow,
} from '#/components/ui/table'
import { DECISION_LABELS, type SourceEvent } from '#/domain/sources/mock/events'
import { taskPathId } from '#/domain/tasks/task-ids'

const TASK_ID = /^NAT-\d+$/

interface Props {
	events: ReadonlyArray<SourceEvent>
}

export function EventsTable({ events }: Props) {
	return (
		<Table>
			<TableHeader>
				<TableRow>
					<TableHead>Reçu</TableHead>
					<TableHead>Valeur</TableHead>
					<TableHead>Motif</TableHead>
					<TableHead>Décision</TableHead>
					<TableHead>Détail</TableHead>
				</TableRow>
			</TableHeader>
			<TableBody>
				{events.map((event) => {
					const decision = DECISION_LABELS[event.decision]
					return (
						<TableRow key={`${event.receivedAt}-${event.value}`}>
							<TableCell className="whitespace-nowrap text-muted-foreground">
								{event.receivedAt}
							</TableCell>
							<TableCell className="font-mono">{event.value}</TableCell>
							<TableCell className="font-mono text-muted-foreground">
								{event.pattern || '—'}
							</TableCell>
							<TableCell>
								<Badge variant={decision.variant}>{decision.label}</Badge>
							</TableCell>
							<TableCell className="text-muted-foreground">
								{TASK_ID.test(event.detail) ? (
									<Link
										to="/tasks/$taskId"
										params={{ taskId: taskPathId(event.detail) }}
										className="font-mono text-link hover:underline"
									>
										{event.detail}
									</Link>
								) : (
									event.detail
								)}
							</TableCell>
						</TableRow>
					)
				})}
			</TableBody>
		</Table>
	)
}
