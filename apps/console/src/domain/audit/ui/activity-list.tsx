import type { ActivityEvent } from '#/domain/audit/mock/audit-data'
import { AgentAvatar, HumanAvatar } from '#/domain/audit/ui/avatar'
import { Panel, Row, Rows, When } from '#/domain/audit/ui/rows'

interface Props {
	events: ReadonlyArray<ActivityEvent>
}

function Actor({ event }: { event: ActivityEvent }) {
	if (event.actor === 'agent') return <AgentAvatar name={event.who} size={22} />
	return (
		<HumanAvatar
			initials={event.actor === 'system' ? 'SY' : (event.initials ?? '')}
			size={22}
		/>
	)
}

export function ActivityList({ events }: Props) {
	return (
		<Panel>
			<Rows>
				{events.map((event) => (
					<Row key={`${event.who}-${event.what}-${event.when}`}>
						<Actor event={event} />
						<span className="min-w-0 flex-1">
							<b className="font-medium">{event.who}</b>{' '}
							<span className="text-xs text-muted-foreground">
								{event.what}
							</span>
						</span>
						<When>{event.when}</When>
					</Row>
				))}
			</Rows>
		</Panel>
	)
}
