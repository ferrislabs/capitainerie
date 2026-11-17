import type { Severity, TimelineEvent } from '#/domain/audit/mock/audit-data'
import { Panel, Row, Rows } from '#/domain/audit/ui/rows'
import { SEV_TONES, type SevTone } from '#/domain/audit/ui/sev'
import { cn } from '#/lib/utils'

interface Props {
	events: ReadonlyArray<TimelineEvent>
}

const SEVERITIES: Record<Severity, { label: string; tone: SevTone }> = {
	critical: { label: 'Critique', tone: 'crit' },
	warning: { label: 'Avertissement', tone: 'warn' },
	info: { label: 'Information', tone: 'info' },
	success: { label: 'Succès', tone: 'ok' },
	created: { label: 'Création', tone: 'vio' },
}

export function TimelineList({ events }: Props) {
	return (
		<Panel>
			<Rows>
				{events.map((event) => (
					<Row key={`${event.time}-${event.text}`}>
						<time className="w-11 font-mono text-[11.5px] text-muted-foreground">
							{event.time}
						</time>
						<span
							role="img"
							aria-label={SEVERITIES[event.severity].label}
							className={cn(
								'size-2.5 shrink-0 rounded-full border',
								SEV_TONES[SEVERITIES[event.severity].tone],
							)}
						/>
						<span className="min-w-0 flex-1 truncate font-medium">
							{event.text}
						</span>
					</Row>
				))}
			</Rows>
		</Panel>
	)
}
