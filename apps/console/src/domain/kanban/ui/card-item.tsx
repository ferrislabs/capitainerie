import type { Schemas } from '#/api/api.client'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { agentName, type Column } from '#/domain/kanban/columns'
import { relativeDate } from '#/domain/kanban/relative-date'
import { LinkChips } from '#/domain/kanban/ui/link-chips'
import { cn } from '#/lib/utils'

interface Props {
	card: Schemas.CardResponse
	isPending: boolean
	expanded: boolean
	onToggleExpanded: (cardId: string) => void
	onMove: (cardId: string, to: Column, expectedVersion: number) => void
	onExecute?: (card: Schemas.CardResponse) => void
	dragging?: 'origin' | 'overlay'
}

export function CardItem({
	card,
	isPending,
	expanded,
	onToggleExpanded,
	onMove,
	onExecute,
	dragging,
}: Props) {
	const agent = card.created_by_agent

	return (
		<article
			id={`card-${card.id}`}
			aria-busy={isPending || undefined}
			className={cn(
				'flex flex-col gap-2 rounded-md border bg-card p-3 transition-opacity duration-150',
				isPending && 'opacity-60',
				dragging === 'origin' &&
					'border-dashed border-input bg-transparent [&>*]:invisible',
				dragging === 'overlay' && 'shadow-float',
			)}
		>
			<div data-zone="title" className="flex flex-col gap-2">
				<h3 className="text-[13px] leading-snug font-semibold break-words [text-wrap:pretty]">
					{card.title}
				</h3>
			</div>
			<div data-zone="meta" className="flex flex-col gap-2">
				{card.description ? (
					<div className="flex flex-col items-start gap-0.5">
						<p
							className={cn(
								'text-xs leading-relaxed text-muted-foreground',
								!expanded && 'line-clamp-3',
							)}
						>
							{card.description}
						</p>
						<Button
							type="button"
							variant="link"
							size="sm"
							className="h-auto p-0 text-xs"
							onClick={() => onToggleExpanded(card.id)}
						>
							{expanded ? 'Réduire' : 'Développer'}
						</Button>
					</div>
				) : null}
				{card.provenance ? (
					<figure className="flex flex-col gap-1 rounded-sm bg-muted/70 px-2 py-1.5">
						<figcaption className="text-[11px] font-medium text-muted-foreground">
							extrait du transcript
						</figcaption>
						<blockquote className="text-xs leading-relaxed italic">
							« {card.provenance.quote} »
						</blockquote>
					</figure>
				) : null}
				<div className="flex min-w-0 flex-wrap items-center gap-1.5 pt-0.5">
					<LinkChips client={card.client} project={card.project} />
					{agent ? (
						<Badge variant="secondary" className="font-mono text-[11px]">
							{agentName(agent)}
						</Badge>
					) : null}
					<span className="text-[11px] text-muted-foreground">
						{relativeDate(card.created_at)}
					</span>
				</div>
			</div>
			{card.column === 'proposed' ? (
				<div data-zone="footer" className="flex items-center gap-2">
					<Button
						type="button"
						size="sm"
						disabled={isPending}
						onClick={() => onMove(card.id, 'todo', card.version)}
					>
						Valider
					</Button>
					<Button
						type="button"
						size="sm"
						variant="ghost"
						className="ml-auto text-signal-danger hover:bg-signal-danger-soft hover:text-signal-danger"
						disabled={isPending}
						onClick={() => onMove(card.id, 'rejected', card.version)}
					>
						Rejeter
					</Button>
				</div>
			) : null}
			{card.column === 'todo' && onExecute ? (
				<div data-zone="footer" className="flex">
					<Button
						type="button"
						size="sm"
						variant="outline"
						className="h-auto min-h-7 w-full py-1 whitespace-normal"
						disabled={isPending}
						onClick={() => onExecute(card)}
					>
						Confier à Claude Code
					</Button>
				</div>
			) : null}
		</article>
	)
}
