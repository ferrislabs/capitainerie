import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Skeleton } from '#/components/ui/skeleton'
import { COLUMNS } from '#/domain/kanban/columns'
import { ExecuteCardFlow } from '#/domain/kanban/feature/execute-card-flow'
import { useCards } from '#/domain/kanban/hooks/use-cards'
import { useMoveCard } from '#/domain/kanban/hooks/use-move-card'
import { KanbanBoard } from '#/domain/kanban/ui/kanban-board'

function BoardSkeleton() {
	return (
		<output className="grid gap-3 lg:grid-flow-col lg:auto-cols-[minmax(11rem,1fr)]">
			<span className="sr-only">Chargement…</span>
			{COLUMNS.map((column) => (
				<div
					key={column}
					className="flex flex-col gap-2 rounded-lg border bg-muted/40 p-2"
				>
					<Skeleton className="m-1 h-3 w-20" />
					<Skeleton className="h-24 w-full rounded-md" />
					<Skeleton className="h-16 w-full rounded-md" />
				</div>
			))}
		</output>
	)
}

export function KanbanPage() {
	const cards = useCards()
	const move = useMoveCard()
	const [expandedIds, setExpandedIds] = useState<ReadonlySet<string>>(new Set())
	const [executingId, setExecutingId] = useState<string>()
	const [startedRunId, setStartedRunId] = useState<string>()

	function toggleExpanded(cardId: string) {
		setExpandedIds((current) => {
			const next = new Set(current)
			if (!next.delete(cardId)) next.add(cardId)
			return next
		})
	}

	return (
		<Page className="max-w-none">
			<PageHeader
				title="Kanban"
				description="Les agents proposent, vous décidez : une carte ne quitte « Proposé » que sur votre validation."
			/>
			{startedRunId ? (
				<InlineAlert tone="success">
					Carte confiée à Claude Code.{' '}
					<Link
						to="/runs/$runId"
						params={{ runId: startedRunId }}
						className="underline underline-offset-2"
					>
						Suivre l’exécution
					</Link>
				</InlineAlert>
			) : null}
			{move.errorMessage ? (
				<InlineAlert tone="danger">{move.errorMessage}</InlineAlert>
			) : null}
			{cards.isError ? (
				<InlineAlert tone="danger">
					Impossible de charger les cartes.
				</InlineAlert>
			) : cards.isPending ? (
				<BoardSkeleton />
			) : (
				<KanbanBoard
					cards={cards.data.items}
					pendingCardId={move.pendingCardId}
					expandedIds={expandedIds}
					onToggleExpanded={toggleExpanded}
					onExecute={(card) => {
						setStartedRunId(undefined)
						setExecutingId(card.id)
					}}
					onMove={(cardId, to, expectedVersion) =>
						move.mutate({
							path: { card_id: cardId },
							body: { to, expected_version: expectedVersion },
						})
					}
				/>
			)}
			<ExecuteCardFlow
				card={cards.data?.items.find((card) => card.id === executingId)}
				onClose={() => setExecutingId(undefined)}
				onStarted={(runId) => {
					setExecutingId(undefined)
					setStartedRunId(runId)
				}}
			/>
		</Page>
	)
}
