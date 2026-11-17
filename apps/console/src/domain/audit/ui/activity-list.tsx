import { SectionCard } from '#/components/layout/section-card'
import type { ActivityEvent } from '#/domain/audit/mock/audit-data'
import { Avatar } from '#/domain/audit/ui/avatar'

interface Props {
	events: ReadonlyArray<ActivityEvent>
}

export function ActivityList({ events }: Props) {
	return (
		<SectionCard title="Activité récente" flush>
			<ul className="divide-y">
				{events.map((event) => (
					<li
						key={`${event.who}-${event.what}-${event.when}`}
						className="flex items-center gap-2.5 px-3 py-2"
					>
						<Avatar name={event.who} />
						<span className="min-w-0 flex-1">
							<b className="font-medium">{event.who}</b>{' '}
							<span className="text-muted-foreground">{event.what}</span>
						</span>
						<span className="shrink-0 text-xs text-muted-foreground">
							{event.when}
						</span>
					</li>
				))}
			</ul>
		</SectionCard>
	)
}
