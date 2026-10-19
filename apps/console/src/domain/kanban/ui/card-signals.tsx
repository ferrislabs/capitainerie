import {
	ArrowUp,
	CalendarClock,
	ListChecks,
	OctagonAlert,
	Tag,
} from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { Badge } from '#/components/ui/badge'
import {
	criteriaProgress,
	formatDueDate,
	initials,
	isFlagged,
	isOverdue,
	PRIORITY_LABELS,
	splitLabels,
} from '#/domain/kanban/card-details'
import { cn } from '#/lib/utils'

interface Props {
	card: Schemas.CardResponse
	assigneeName?: string | undefined
	now?: Date
}

export function PriorityBadge({ priority }: { priority: Schemas.PriorityDto }) {
	if (!isFlagged(priority)) return null
	const urgent = priority === 'urgent'
	const Icon = urgent ? OctagonAlert : ArrowUp
	return (
		<Badge variant={urgent ? 'danger' : 'neutral'} data-priority={priority}>
			<Icon aria-hidden="true" className="size-3 shrink-0" />
			<span className="sr-only">Priorité : </span>
			{PRIORITY_LABELS[priority]}
		</Badge>
	)
}

export function DueDate({
	dueDate,
	now,
}: {
	dueDate: string
	now?: Date | undefined
}) {
	const late = isOverdue(dueDate, now)
	return (
		<span
			data-overdue={late || undefined}
			className={cn(
				'inline-flex items-center gap-1 text-[11px] figures',
				late ? 'font-medium text-signal-danger' : 'text-muted-foreground',
			)}
		>
			<CalendarClock aria-hidden="true" className="size-3 shrink-0" />
			<span className="sr-only">Échéance : </span>
			<time dateTime={dueDate}>{formatDueDate(dueDate)}</time>
			{late ? <span>· en retard</span> : null}
		</span>
	)
}

export function AssigneeInitials({ name }: { name: string }) {
	return (
		<span
			role="img"
			aria-label={`Responsable : ${name}`}
			title={name}
			className="inline-flex size-5 shrink-0 items-center justify-center rounded-sm bg-secondary text-[10px] font-semibold text-secondary-foreground"
		>
			{initials(name)}
		</span>
	)
}

export function CardSignals({ card, assigneeName, now }: Props) {
	const progress = criteriaProgress(card.acceptance_criteria)
	const { shown, hidden } = splitLabels(card.labels)
	const hasAny =
		isFlagged(card.priority) ||
		card.due_date != null ||
		card.assignee_subject != null ||
		shown.length > 0 ||
		progress !== null
	if (!hasAny) return null

	return (
		<div
			data-testid="card-signals"
			className="flex min-w-0 flex-wrap items-center gap-1.5"
		>
			<PriorityBadge priority={card.priority} />
			{card.due_date ? <DueDate dueDate={card.due_date} now={now} /> : null}
			{progress ? (
				<span className="inline-flex items-center gap-1 text-[11px] text-muted-foreground figures">
					<ListChecks aria-hidden="true" className="size-3 shrink-0" />
					<span className="sr-only">Critères remplis : </span>
					{progress.done}/{progress.total}
				</span>
			) : null}
			{shown.map((label) => (
				<Badge key={label} variant="outline" className="max-w-full">
					<Tag aria-hidden="true" className="size-3 shrink-0" />
					<span className="sr-only">Étiquette : </span>
					<span className="truncate">{label}</span>
				</Badge>
			))}
			{hidden > 0 ? (
				<Badge
					variant="muted"
					title={card.labels.slice(shown.length).join(', ')}
				>
					+{hidden}
					<span className="sr-only"> autres étiquettes</span>
				</Badge>
			) : null}
			{card.assignee_subject ? (
				<span className="ml-auto">
					<AssigneeInitials name={assigneeName ?? card.assignee_subject} />
				</span>
			) : null}
		</div>
	)
}
