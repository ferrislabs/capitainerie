import { useId } from 'react'
import type { Schemas } from '#/api/api.client'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { Card, CardContent } from '#/components/ui/card'
import {
	agentName,
	COLUMN_LABELS,
	COLUMNS,
	type Column,
} from '#/domain/kanban/columns'
import { relativeDate } from '#/domain/kanban/relative-date'

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
		<Card className="gap-2 py-3">
			<CardContent className="flex flex-col gap-2 px-3">
				<div className="flex items-start justify-between gap-2">
					<h3 className="min-w-0 break-words text-sm font-medium">
						{card.title}
					</h3>
					{agent ? (
						<Badge variant="secondary" className="shrink-0">
							{agentName(agent)}
						</Badge>
					) : null}
				</div>
				{card.description ? (
					<div className="flex flex-col items-start gap-1">
						<p
							className={
								expanded
									? 'text-xs text-muted-foreground'
									: 'line-clamp-3 text-xs text-muted-foreground'
							}
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
					<div className="flex flex-col gap-1">
						<span className="text-[10px] uppercase tracking-wide text-muted-foreground">
							extrait du transcript
						</span>
						<blockquote className="border-l-2 pl-2 text-xs italic">
							« {card.provenance.quote} »
						</blockquote>
					</div>
				) : null}
				<span className="text-xs text-muted-foreground">
					{relativeDate(card.created_at)}
				</span>
				{card.column === 'proposed' ? (
					<div className="flex gap-2">
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
							variant="outline"
							disabled={isPending}
							onClick={() => onMove(card.id, 'rejected', card.version)}
						>
							Rejeter
						</Button>
					</div>
				) : (
					<select
						id={selectId}
						aria-label="Déplacer vers…"
						className="h-8 rounded-md border bg-background px-2 text-xs disabled:opacity-50"
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
				)}
			</CardContent>
		</Card>
	)
}
