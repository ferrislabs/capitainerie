import { Link } from '@tanstack/react-router'
import { SearchX } from 'lucide-react'
import { useState } from 'react'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { Skeleton } from '#/components/ui/skeleton'
import { InterveneCardFlow } from '#/domain/integrations/intervene/intervene-card-flow'
import { memberLabels } from '#/domain/kanban/card-details'
import {
	type CardFilters,
	filterCards,
	filterOptions,
	hasFilters,
	NO_FILTERS,
} from '#/domain/kanban/card-filters'
import { COLUMNS } from '#/domain/kanban/columns'
import { CardPanel } from '#/domain/kanban/feature/card-panel'
import { ExecuteCardFlow } from '#/domain/kanban/feature/execute-card-flow'
import { useCards } from '#/domain/kanban/hooks/use-cards'
import { memberNames, useMembers } from '#/domain/kanban/hooks/use-members'
import { useMoveCard } from '#/domain/kanban/hooks/use-move-card'
import { KanbanBoard } from '#/domain/kanban/ui/kanban-board'
import { KanbanToolbar } from '#/domain/kanban/ui/kanban-toolbar'

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

interface StartedRun {
	runId: string
	by: 'claude' | 'intervener'
}

interface Props {
	openCardId: string | undefined
	onOpenCard: (cardId: string | undefined) => void
	filters?: CardFilters
	onFiltersChange?: (filters: CardFilters) => void
}

export function KanbanPage({
	openCardId,
	onOpenCard,
	filters = NO_FILTERS,
	onFiltersChange = () => undefined,
}: Props) {
	const cards = useCards()
	const move = useMoveCard()
	const members = useMembers()
	const [expandedIds, setExpandedIds] = useState<ReadonlySet<string>>(new Set())
	const [executingId, setExecutingId] = useState<string>()
	const [intervenerId, setIntervenerId] = useState<string>()
	const [started, setStarted] = useState<StartedRun>()

	const all = cards.data?.items ?? []
	const names = memberNames(members.data?.items ?? [])
	const assigneeLabels = memberLabels(members.data?.items ?? [])
	const visible = filterCards(all, filters)
	const options = filterOptions(all, filters, assigneeLabels)

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
			{started ? (
				<InlineAlert tone="success">
					{started.by === 'claude'
						? 'Carte confiée à Claude Code. '
						: 'Carte confiée à l’Intervenant. '}
					<Link
						to="/runs/$runId"
						params={{ runId: started.runId }}
						className="underline underline-offset-2"
					>
						{started.by === 'claude'
							? 'Suivre l’exécution'
							: 'Suivre l’intervention'}
					</Link>
				</InlineAlert>
			) : null}
			<KanbanToolbar
				filters={filters}
				options={options}
				assigneeLabels={assigneeLabels}
				onChange={onFiltersChange}
			/>
			{move.errorMessage ? (
				<InlineAlert tone="danger">{move.errorMessage}</InlineAlert>
			) : null}
			{cards.isError ? (
				<InlineAlert tone="danger">
					Impossible de charger les cartes.
				</InlineAlert>
			) : cards.isPending ? (
				<BoardSkeleton />
			) : visible.length === 0 && hasFilters(filters) ? (
				<EmptyState
					icon={SearchX}
					title="Aucune carte"
					description="Aucune carte ne correspond à ces filtres."
					action={
						<Button
							type="button"
							variant="outline"
							onClick={() => onFiltersChange(NO_FILTERS)}
						>
							Réinitialiser les filtres
						</Button>
					}
				/>
			) : (
				<KanbanBoard
					cards={visible}
					memberNames={names}
					pendingCardId={move.pendingCardId}
					expandedIds={expandedIds}
					onToggleExpanded={toggleExpanded}
					onOpen={(cardId) => onOpenCard(cardId)}
					onExecute={(card) => {
						setStarted(undefined)
						setExecutingId(card.id)
					}}
					onIntervene={(card) => {
						setStarted(undefined)
						setIntervenerId(card.id)
					}}
					onMove={(cardId, to, expectedVersion) =>
						move.mutate({
							path: { card_id: cardId },
							body: { to, expected_version: expectedVersion },
						})
					}
				/>
			)}
			<CardPanel
				key={openCardId ?? 'closed'}
				cardId={openCardId}
				listedCard={cards.data?.items.find((card) => card.id === openCardId)}
				isMoving={move.pendingCardId === openCardId}
				moveError={move.errorMessage}
				onClose={() => onOpenCard(undefined)}
				onMove={(cardId, to, expectedVersion) =>
					move.mutate({
						path: { card_id: cardId },
						body: { to, expected_version: expectedVersion },
					})
				}
				onExecute={(card) => {
					setStarted(undefined)
					onOpenCard(undefined)
					setExecutingId(card.id)
				}}
				onIntervene={(card) => {
					setStarted(undefined)
					onOpenCard(undefined)
					setIntervenerId(card.id)
				}}
			/>
			<ExecuteCardFlow
				card={cards.data?.items.find((card) => card.id === executingId)}
				onClose={() => setExecutingId(undefined)}
				onStarted={(runId) => {
					setExecutingId(undefined)
					setStarted({ runId, by: 'claude' })
				}}
			/>
			<InterveneCardFlow
				card={cards.data?.items.find((card) => card.id === intervenerId)}
				onClose={() => setIntervenerId(undefined)}
				onStarted={(runId) => {
					setIntervenerId(undefined)
					setStarted({ runId, by: 'intervener' })
				}}
			/>
		</Page>
	)
}
