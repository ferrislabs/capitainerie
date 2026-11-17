import { SectionCard } from '#/components/layout/section-card'
import type { Severity, TimelineEvent } from '#/domain/audit/mock/audit-data'
import { cn } from '#/lib/utils'

interface Props {
	events: ReadonlyArray<TimelineEvent>
}

const SEVERITIES: Record<Severity, { label: string; dot: string }> = {
	critical: { label: 'Critique', dot: 'bg-signal-danger' },
	warning: { label: 'Avertissement', dot: 'bg-signal-attention' },
	info: { label: 'Information', dot: 'bg-signal-running' },
	success: { label: 'Succès', dot: 'bg-signal-success' },
	created: { label: 'Création', dot: 'bg-signal-neutral' },
}

export function TimelineList({ events }: Props) {
	return (
		<SectionCard title="Événements notables" flush>
			<ol className="divide-y">
				{events.map((event) => (
					<li
						key={`${event.time}-${event.text}`}
						className="flex items-center gap-3 px-3 py-2"
					>
						<time className="figures w-11 text-muted-foreground">
							{event.time}
						</time>
						<span
							role="img"
							aria-label={SEVERITIES[event.severity].label}
							className={cn(
								'size-2.5 shrink-0 rounded-full',
								SEVERITIES[event.severity].dot,
							)}
						/>
						<span className="min-w-0 flex-1">{event.text}</span>
					</li>
				))}
			</ol>
		</SectionCard>
	)
}
