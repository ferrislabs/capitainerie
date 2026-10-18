import { ChevronDown } from 'lucide-react'
import { useId } from 'react'
import type { Schemas } from '#/api/api.client'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import {
	agentName,
	COLUMN_LABELS,
	COLUMNS,
	type Column,
} from '#/domain/kanban/columns'
import { relativeDate } from '#/domain/kanban/relative-date'
import { cn } from '#/lib/utils'

interface Props {
	card: Schemas.CardResponse
	isPending: boolean
	expanded: boolean
	onToggleExpanded: (cardId: string) => void
	onMove: (cardId: string, to: Column, expectedVersion: number) => void
}

export function CardItem({
	card,
	isPending,
	expanded,
	onToggleExpanded,
	onMove,
}: Props) {
	const selectId = useId()
	const agent = card.created_by_agent

	return (
		<article
			aria-busy={isPending || undefined}
			className={cn(
				'flex flex-col gap-2 rounded-md border bg-card p-3 transition-opacity duration-150',
				isPending && 'opacity-60',
			)}
		>
			<h3 className="text-[13px] leading-snug font-semibold break-words [text-wrap:pretty]">
				{card.title}
			</h3>
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
			<div className="flex flex-wrap items-center justify-between gap-2 pt-0.5">
				<div className="flex min-w-0 items-center gap-1.5">
					{agent ? (
						<Badge variant="secondary" className="font-mono text-[11px]">
							{agentName(agent)}
						</Badge>
					) : null}
					<span className="text-[11px] text-muted-foreground">
						{relativeDate(card.created_at)}
					</span>
				</div>
				{card.column === 'proposed' ? (
					<div className="flex w-full items-center gap-2">
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
				) : (
					<div className="relative">
						<select
							id={selectId}
							aria-label="Déplacer vers…"
							className="h-7 appearance-none rounded-md border border-input bg-card pr-7 pl-2 text-xs text-foreground focus-visible:border-ring focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/25 disabled:opacity-50"
							disabled={isPending}
							value=""
							onChange={(event) => {
								if (event.target.value) {
									onMove(card.id, event.target.value as Column, card.version)
								}
							}}
						>
							<option value="">Déplacer vers…</option>
							{COLUMNS.filter((column) => column !== card.column).map(
								(column) => (
									<option key={column} value={column}>
										{COLUMN_LABELS[column]}
									</option>
								),
							)}
						</select>
						<ChevronDown
							aria-hidden="true"
							className="pointer-events-none absolute top-1/2 right-2 size-3.5 -translate-y-1/2 text-muted-foreground"
						/>
					</div>
				)}
			</div>
		</article>
	)
}
