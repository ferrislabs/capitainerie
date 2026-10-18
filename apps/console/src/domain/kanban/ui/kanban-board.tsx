import { KanbanSquare } from 'lucide-react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import { Badge } from '#/components/ui/badge'
import { COLUMN_LABELS, COLUMNS, type Column } from '#/domain/kanban/columns'
import { CardItem } from '#/domain/kanban/ui/card-item'
import { cn } from '#/lib/utils'

interface Props {
	cards: ReadonlyArray<Schemas.CardResponse>
	pendingCardId: string | undefined
	expandedIds: ReadonlySet<string>
	onToggleExpanded: (cardId: string) => void
	onMove: (cardId: string, to: Column, expectedVersion: number) => void
}

export function KanbanBoard({
	cards,
	pendingCardId,
	expandedIds,
	onToggleExpanded,
	onMove,
}: Props) {
	if (cards.length === 0) {
		return (
			<EmptyState
				icon={KanbanSquare}
				title="Aucune carte"
				description="Aucune carte. Déposez un transcript : les agents proposeront des cartes ici."
			/>
		)
	}

	return (
		<div className="grid gap-3 pb-2 lg:grid-flow-col lg:auto-cols-[minmax(11rem,1fr)] lg:items-start lg:overflow-x-auto">
			{COLUMNS.map((column) => {
				const items = cards.filter((card) => card.column === column)
				const highlighted = column === 'proposed'
				return (
					<section
						key={column}
						aria-label={COLUMN_LABELS[column]}
						data-column={column}
						className={cn(
							'flex min-w-0 flex-col overflow-hidden rounded-lg border lg:max-h-[calc(100dvh-15rem)]',
							highlighted
								? 'border-signal-attention/70 bg-signal-attention-soft/60'
								: 'bg-muted/50',
						)}
					>
						<header
							className={cn(
								'flex h-10 shrink-0 items-center justify-between gap-2 px-3',
								highlighted
									? 'border-b border-signal-attention/50 bg-signal-attention-soft'
									: 'border-b',
							)}
						>
							<div className="flex items-center gap-2">
								{highlighted ? (
									<span
										aria-hidden="true"
										className="size-2 rounded-full bg-signal-attention"
									/>
								) : null}
								<h2 className="text-[13px] font-semibold">
									{COLUMN_LABELS[column]}
								</h2>
							</div>
							{highlighted ? (
								<Badge
									variant={items.length > 0 ? 'signal' : 'outline'}
									data-testid="proposed-count"
								>
									{items.length}
								</Badge>
							) : (
								<span className="text-xs text-muted-foreground figures">
									{items.length}
								</span>
							)}
						</header>
						<div className="flex flex-col gap-2 overflow-x-hidden overflow-y-auto p-2">
							{items.length === 0 ? (
								<p className="px-1 py-3 text-center text-xs text-muted-foreground">
									Vide
								</p>
							) : null}
							{items.map((card) => (
								<CardItem
									key={card.id}
									card={card}
									isPending={pendingCardId === card.id}
									expanded={expandedIds.has(card.id)}
									onToggleExpanded={onToggleExpanded}
									onMove={onMove}
								/>
							))}
						</div>
					</section>
				)
			})}
		</div>
	)
}
