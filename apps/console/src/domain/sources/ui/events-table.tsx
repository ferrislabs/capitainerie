import { Link } from '@tanstack/react-router'
import { DECISION_LABELS, type SourceEvent } from '#/domain/sources/mock/events'
import { Sev, Tbl, Td, Th, Tr } from '#/domain/sources/ui/mockup'
import { taskPathId } from '#/domain/tasks/task-ids'

const TASK_ID = /^NAT-\d+$/

interface Props {
	events: ReadonlyArray<SourceEvent>
}

export function EventsTable({ events }: Props) {
	return (
		<Tbl>
			<thead>
				<tr>
					<Th>Reçu</Th>
					<Th>Valeur</Th>
					<Th>Motif</Th>
					<Th>Décision</Th>
					<Th>Détail</Th>
				</tr>
			</thead>
			<tbody>
				{events.map((event) => {
					const decision = DECISION_LABELS[event.decision]
					return (
						<Tr key={`${event.receivedAt}-${event.value}`}>
							<Td className="whitespace-nowrap text-muted-foreground">
								{event.receivedAt}
							</Td>
							<Td className="font-mono text-[11.5px]">{event.value}</Td>
							<Td className="font-mono text-[11.5px] text-muted-foreground">
								{event.pattern || '—'}
							</Td>
							<Td>
								<Sev tone={decision.tone}>{decision.label}</Sev>
							</Td>
							<Td>
								{TASK_ID.test(event.detail) ? (
									<Link
										to="/tasks/$taskId"
										params={{ taskId: taskPathId(event.detail) }}
										className="text-[12px] text-primary"
									>
										{event.detail}
									</Link>
								) : (
									<span className="text-muted-foreground">{event.detail}</span>
								)}
							</Td>
						</Tr>
					)
				})}
			</tbody>
		</Tbl>
	)
}
