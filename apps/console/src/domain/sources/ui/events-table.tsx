import { Link } from '@tanstack/react-router'
import { relativeAge } from '#/domain/dashboard/format'
import { EVENT_STATUS, type SourceEvent } from '#/domain/sources/model'
import { Sev, Tbl, Td, Th, Tr } from '#/domain/sources/ui/mockup'
import { shortTaskId, taskPathId } from '#/domain/tasks/task-ids'
import { cn } from '#/lib/utils'

interface Props {
	events: ReadonlyArray<SourceEvent>
}

export function EventsTable({ events }: Props) {
	return (
		<Tbl>
			<thead>
				<tr>
					<Th>Reçu</Th>
					<Th>Résumé</Th>
					<Th>Statut</Th>
					<Th>Tâche</Th>
					<Th>Détail</Th>
				</tr>
			</thead>
			<tbody>
				{events.map((event) => {
					const status = EVENT_STATUS[event.status]
					return (
						<Tr key={event.id}>
							<Td className="whitespace-nowrap text-muted-foreground">
								<time
									dateTime={event.received_at}
									title={new Date(event.received_at).toLocaleString('fr-FR')}
								>
									{relativeAge(event.received_at)}
								</time>
							</Td>
							<Td className="[overflow-wrap:anywhere]">
								{event.summary || '—'}
							</Td>
							<Td>
								<Sev tone={status.tone}>{status.label}</Sev>
							</Td>
							<Td>
								{event.card_id ? (
									<Link
										to="/tasks/$taskId"
										params={{ taskId: taskPathId(event.card_id) }}
										className="font-mono text-[12px] text-primary"
									>
										{shortTaskId(event.card_id)}
									</Link>
								) : (
									<span className="text-muted-foreground">—</span>
								)}
							</Td>
							<Td
								className={cn(
									'max-w-80 text-[12px] [overflow-wrap:anywhere]',
									event.status === 'failed'
										? 'text-signal-danger'
										: 'text-muted-foreground',
								)}
							>
								{event.error ?? ''}
							</Td>
						</Tr>
					)
				})}
			</tbody>
		</Tbl>
	)
}
