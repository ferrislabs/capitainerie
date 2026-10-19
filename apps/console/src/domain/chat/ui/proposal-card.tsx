import { Link } from '@tanstack/react-router'
import { Building2, FolderKanban, ListChecks } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import type { ProposalView } from '#/domain/chat/chat-state'
import { ESTIMATE_LABELS, PRIORITY_LABELS } from '#/domain/kanban/card-details'
import { COLUMN_LABELS } from '#/domain/kanban/columns'

interface Props {
	proposal: ProposalView
	card: Schemas.CardResponse | undefined
	pending: boolean
	errorMessage?: string
	onValidate: (card: Schemas.CardResponse) => void
	onReject: (card: Schemas.CardResponse) => void
	onExecute: (card: Schemas.CardResponse) => void
}

const COLUMN_BADGES: Record<
	Schemas.ColumnDto,
	'attention' | 'neutral' | 'running' | 'success' | 'danger'
> = {
	proposed: 'attention',
	todo: 'neutral',
	in_progress: 'running',
	done: 'success',
	rejected: 'danger',
}

const PRIORITY_BADGES: Record<
	Schemas.PriorityDto,
	'danger' | 'attention' | 'muted' | undefined
> = {
	urgent: 'danger',
	high: 'attention',
	normal: undefined,
	low: 'muted',
}

function Details({ card }: { card: Schemas.CardResponse }) {
	const priority = PRIORITY_BADGES[card.priority]
	const criteria = card.acceptance_criteria.length
	if (
		!card.client &&
		!card.project &&
		!priority &&
		!card.estimate &&
		criteria === 0
	) {
		return null
	}
	return (
		<ul
			aria-label="Détails de la carte"
			className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground"
		>
			{card.client ? (
				<li className="inline-flex items-center gap-1">
					<Building2 aria-hidden="true" className="size-3 shrink-0" />
					<span className="sr-only">Client : </span>
					{card.client.name}
				</li>
			) : null}
			{card.project ? (
				<li className="inline-flex items-center gap-1">
					<FolderKanban aria-hidden="true" className="size-3 shrink-0" />
					<span className="sr-only">Projet : </span>
					{card.project.name}
				</li>
			) : null}
			{priority ? (
				<li>
					<Badge variant={priority}>
						Priorité {PRIORITY_LABELS[card.priority].toLowerCase()}
					</Badge>
				</li>
			) : null}
			{card.estimate ? (
				<li className="figures">Estimation {ESTIMATE_LABELS[card.estimate]}</li>
			) : null}
			{criteria > 0 ? (
				<li className="inline-flex items-center gap-1 figures">
					<ListChecks aria-hidden="true" className="size-3 shrink-0" />
					{criteria} {criteria > 1 ? 'critères' : 'critère'}
				</li>
			) : null}
		</ul>
	)
}

export function ProposalCard({
	proposal,
	card,
	pending,
	errorMessage,
	onValidate,
	onReject,
	onExecute,
}: Props) {
	return (
		<article
			aria-busy={pending || undefined}
			aria-label={`Carte proposée : ${proposal.title}`}
			className="flex max-w-xl flex-col gap-2 rounded-md border bg-card p-3"
		>
			<div className="flex flex-col gap-1">
				<div className="flex flex-wrap items-center gap-2">
					<h3 className="text-[13px] leading-snug font-semibold break-words">
						{card?.title ?? proposal.title}
					</h3>
					{card ? (
						<Badge variant={COLUMN_BADGES[card.column]}>
							{card.column === 'proposed'
								? 'À valider'
								: COLUMN_LABELS[card.column]}
						</Badge>
					) : null}
				</div>
				{card?.description ? (
					<p className="line-clamp-3 text-xs leading-relaxed text-muted-foreground">
						{card.description}
					</p>
				) : null}
				{card ? <Details card={card} /> : null}
			</div>
			<div className="flex flex-wrap items-center gap-2">
				{card?.column === 'proposed' ? (
					<>
						<Button
							type="button"
							size="sm"
							disabled={pending}
							onClick={() => onValidate(card)}
						>
							Valider
						</Button>
						<Button
							type="button"
							size="sm"
							variant="outline"
							disabled={pending}
							onClick={() => onExecute(card)}
						>
							Confier à Claude Code
						</Button>
						<Button
							type="button"
							size="sm"
							variant="ghost"
							className="text-signal-danger hover:bg-signal-danger-soft hover:text-signal-danger"
							disabled={pending}
							onClick={() => onReject(card)}
						>
							Rejeter
						</Button>
					</>
				) : null}
				{card?.column === 'todo' ? (
					<Button
						type="button"
						size="sm"
						variant="outline"
						disabled={pending}
						onClick={() => onExecute(card)}
					>
						Confier à Claude Code
					</Button>
				) : null}
				<Button asChild size="sm" variant="ghost" className="ml-auto">
					<Link to="/kanban" search={{ card: proposal.cardId }}>
						Ouvrir
					</Link>
				</Button>
			</div>
			{errorMessage ? (
				<p role="alert" className="text-xs text-signal-danger">
					{errorMessage}
				</p>
			) : null}
		</article>
	)
}
