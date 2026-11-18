import {
	type AuditEvent,
	groupByDay,
	SEVERITIES,
	timeOf,
} from '#/domain/audit/model'
import { Eyebrow, Panel, PanelEmpty, Row, Rows } from '#/domain/audit/ui/rows'
import { SEV_TONES } from '#/domain/audit/ui/sev'
import { cn } from '#/lib/utils'

interface Props {
	events: ReadonlyArray<AuditEvent>
}

export function TimelineList({ events }: Props) {
	if (events.length === 0) {
		return (
			<Panel>
				<PanelEmpty title="Chronologie vide">
					Aucun événement enregistré pour le moment.
				</PanelEmpty>
			</Panel>
		)
	}
	return (
		<>
			{groupByDay(events).map((group) => (
				<section key={group.day} aria-label={group.day}>
					<Eyebrow>{group.day}</Eyebrow>
					<Panel>
						<Rows>
							{group.events.map((event) => {
								const severity = SEVERITIES[event.severity]
								return (
									<Row key={event.id}>
										<time
											dateTime={event.occurred_at}
											className="w-11 font-mono text-[11.5px] text-muted-foreground"
										>
											{timeOf(event.occurred_at)}
										</time>
										<span
											role="img"
											aria-label={severity.label}
											className={cn(
												'size-2.5 shrink-0 rounded-full border',
												SEV_TONES[severity.tone],
											)}
										/>
										<span className="min-w-0 flex-1 truncate font-medium">
											{event.summary}
										</span>
										<span className="truncate text-xs text-muted-foreground">
											{event.actor.name}
										</span>
									</Row>
								)
							})}
						</Rows>
					</Panel>
				</section>
			))}
		</>
	)
}
