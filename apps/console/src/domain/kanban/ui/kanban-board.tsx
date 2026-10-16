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
		<div className="grid gap-4 pb-2 lg:grid-flow-col lg:auto-cols-[minmax(15rem,1fr)] lg:items-start lg:overflow-x-auto">
			{COLUMNS.map((column) => {
				const items = cards.filter((card) => card.column === column)
				const highlighted = column === 'proposed'
				return (
					<section
						key={column}
						aria-label={COLUMN_LABELS[column]}
						data-column={column}
						className={cn(
							'flex min-w-0 flex-col gap-3 rounded-lg border p-3 lg:max-h-[calc(100vh-10rem)]',
							highlighted
								? 'border-primary bg-primary/5 ring-1 ring-primary/30'
								: 'bg-muted/30',
						)}
					>
						<header className="flex items-center justify-between">
							<h2 className="text-sm font-semibold">{COLUMN_LABELS[column]}</h2>
							{highlighted ? (
								<Badge data-testid="proposed-count">{items.length}</Badge>
							) : null}
						</header>
						<div className="flex flex-col gap-2 overflow-x-hidden overflow-y-auto">
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
