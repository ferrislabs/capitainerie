import { type AuditEvent, initials } from '#/domain/audit/model'
import { HumanAvatar } from '#/domain/audit/ui/avatar'
import { Panel, PanelEmpty, Row, Rows, When } from '#/domain/audit/ui/rows'
import { relativeAge } from '#/domain/dashboard/format'
import { AgentAvatar } from '#/domain/tasks/ui/agent-chip'

interface Props {
	events: ReadonlyArray<AuditEvent>
	hueOf: (id: string | null | undefined, name: string) => number
}

function Actor({ event, hueOf }: { event: AuditEvent } & Pick<Props, 'hueOf'>) {
	const { actor } = event
	if (actor.kind === 'agent')
		return <AgentAvatar hue={hueOf(actor.id, actor.name)} size={22} />
	return (
		<HumanAvatar
			initials={actor.kind === 'system' ? 'SY' : initials(actor.name)}
			size={22}
		/>
	)
}

export function ActivityList({ events, hueOf }: Props) {
	return (
		<Panel>
			{events.length === 0 ? (
				<PanelEmpty title="Aucun événement">
					Le journal se remplit au fil des actions des agents et des membres.
				</PanelEmpty>
			) : (
				<Rows>
					{events.map((event) => (
						<Row key={event.id}>
							<Actor event={event} hueOf={hueOf} />
							<span className="min-w-0 flex-1">
								<b className="font-medium">{event.actor.name}</b>{' '}
								<span className="text-xs text-muted-foreground">
									{event.summary}
								</span>
							</span>
							<When>
								<time
									dateTime={event.occurred_at}
									title={new Date(event.occurred_at).toLocaleString('fr-FR')}
								>
									{relativeAge(event.occurred_at)}
								</time>
							</When>
						</Row>
					))}
				</Rows>
			)}
		</Panel>
	)
}
