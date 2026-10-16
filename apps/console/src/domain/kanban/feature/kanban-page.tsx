import { useState } from 'react'
import { useCards } from '#/domain/kanban/hooks/use-cards'
import { useMoveCard } from '#/domain/kanban/hooks/use-move-card'
import { KanbanBoard } from '#/domain/kanban/ui/kanban-board'

export function KanbanPage() {
	const cards = useCards()
	const move = useMoveCard()
	const [expandedIds, setExpandedIds] = useState<ReadonlySet<string>>(new Set())

	function toggleExpanded(cardId: string) {
		setExpandedIds((current) => {
			const next = new Set(current)
			if (!next.delete(cardId)) next.add(cardId)
			return next
		})
	}

	return (
		<section className="flex flex-col gap-4">
			<h1 className="text-2xl font-semibold">Kanban</h1>
			{move.errorMessage ? (
				<p role="alert" className="text-sm text-destructive">
					{move.errorMessage}
				</p>
			) : null}
			{cards.isError ? (
				<p role="alert" className="text-sm text-destructive">
					Impossible de charger les cartes.
				</p>
			) : cards.isPending ? (
				<p className="text-sm text-muted-foreground">Chargement…</p>
			) : (
				<KanbanBoard
					cards={cards.data.items}
					pendingCardId={move.pendingCardId}
					expandedIds={expandedIds}
					onToggleExpanded={toggleExpanded}
					onMove={(cardId, to, expectedVersion) =>
						move.mutate({
							path: { card_id: cardId },
							body: { to, expected_version: expectedVersion },
						})
					}
				/>
			)}
		</section>
	)
}
