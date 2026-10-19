import {
	type CollisionDetection,
	closestCorners,
	DndContext,
	type DragEndEvent,
	type DragOverEvent,
	DragOverlay,
	type DragStartEvent,
	KeyboardCode,
	KeyboardSensor,
	MouseSensor,
	pointerWithin,
	TouchSensor,
	useDroppable,
	useSensor,
	useSensors,
} from '@dnd-kit/core'
import { SortableContext, sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { KanbanSquare } from 'lucide-react'
import { useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import { Badge } from '#/components/ui/badge'
import { canDrop } from '#/domain/kanban/can-drop'
import { COLUMN_LABELS, COLUMNS, type Column } from '#/domain/kanban/columns'
import { columnDropId, columnOfData } from '#/domain/kanban/drag-data'
import { resolveDrop } from '#/domain/kanban/drop'
import {
	createAnnouncements,
	SCREEN_READER_INSTRUCTIONS,
} from '#/domain/kanban/move-announcements'
import { CardItem } from '#/domain/kanban/ui/card-item'
import { DraggableCard } from '#/domain/kanban/ui/draggable-card'
import { cn } from '#/lib/utils'

interface Props {
	cards: ReadonlyArray<Schemas.CardResponse>
	pendingCardId: string | undefined
	expandedIds: ReadonlySet<string>
	onToggleExpanded: (cardId: string) => void
	onMove: (cardId: string, to: Column, expectedVersion: number) => void
	onExecute?: (card: Schemas.CardResponse) => void
	onOpen: (cardId: string) => void
}

const collisionDetection: CollisionDetection = (args) => {
	const within = pointerWithin(args)
	return within.length > 0 ? within : closestCorners(args)
}

const KEYBOARD_CODES = {
	start: [KeyboardCode.Space],
	cancel: [KeyboardCode.Esc],
	end: [KeyboardCode.Space],
}

const noShift = () => null

interface ColumnProps {
	column: Column
	items: ReadonlyArray<Schemas.CardResponse>
	lit: boolean
	children: React.ReactNode
}

function BoardColumn({ column, items, lit, children }: ColumnProps) {
	const { setNodeRef } = useDroppable({
		id: columnDropId(column),
		data: { column },
	})
	const highlighted = column === 'proposed'

	return (
		<section
			ref={setNodeRef}
			aria-label={COLUMN_LABELS[column]}
			data-column={column}
			data-drop-target={lit || undefined}
			className={cn(
				'flex min-w-0 flex-col overflow-hidden rounded-lg border transition-colors duration-150 lg:max-h-[calc(100dvh-15rem)]',
				highlighted
					? 'border-signal-attention/70 bg-signal-attention-soft/60'
					: 'bg-muted/50',
				lit &&
					'border-signal-running bg-signal-running-soft ring-2 ring-signal-running/30',
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
					<h2 className="text-[13px] font-semibold">{COLUMN_LABELS[column]}</h2>
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
			<div className="flex min-h-16 flex-col gap-2 overflow-x-hidden overflow-y-auto p-2">
				{items.length === 0 ? (
					<p className="px-1 py-3 text-center text-xs text-muted-foreground">
						Vide
					</p>
				) : null}
				{children}
			</div>
		</section>
	)
}

function focusCard(cardId: string) {
	requestAnimationFrame(() => {
		document
			.querySelector<HTMLElement>(`[data-card-id="${CSS.escape(cardId)}"]`)
			?.focus()
	})
}

export function KanbanBoard({
	cards,
	pendingCardId,
	expandedIds,
	onToggleExpanded,
	onMove,
	onExecute,
	onOpen,
}: Props) {
	const sensors = useSensors(
		useSensor(MouseSensor, { activationConstraint: { distance: 4 } }),
		useSensor(TouchSensor, {
			activationConstraint: { delay: 200, tolerance: 8 },
		}),
		useSensor(KeyboardSensor, {
			coordinateGetter: sortableKeyboardCoordinates,
			keyboardCodes: KEYBOARD_CODES,
		}),
	)
	const [activeId, setActiveId] = useState<string>()
	const [overColumn, setOverColumn] = useState<Column>()

	if (cards.length === 0) {
		return (
			<EmptyState
				icon={KanbanSquare}
				title="Aucune carte"
				description="Aucune carte. Déposez un transcript : les agents proposeront des cartes ici."
			/>
		)
	}

	const active = cards.find((card) => card.id === activeId)

	function reset() {
		setActiveId(undefined)
		setOverColumn(undefined)
	}

	function handleDragStart(event: DragStartEvent) {
		setActiveId(String(event.active.id))
	}

	function handleDragOver(event: DragOverEvent) {
		setOverColumn(columnOfData(event.over?.data.current))
	}

	function handleDragEnd(event: DragEndEvent) {
		const drop = resolveDrop(cards, event.active.id, event.over?.data.current)
		reset()
		if (drop) onMove(drop.cardId, drop.to, drop.expectedVersion)
		if (event.activatorEvent instanceof KeyboardEvent) {
			focusCard(String(event.active.id))
		}
	}

	return (
		<DndContext
			sensors={sensors}
			collisionDetection={collisionDetection}
			accessibility={{
				announcements: createAnnouncements(cards),
				screenReaderInstructions: { draggable: SCREEN_READER_INSTRUCTIONS },
			}}
			onDragStart={handleDragStart}
			onDragOver={handleDragOver}
			onDragEnd={handleDragEnd}
			onDragCancel={reset}
		>
			<div className="grid gap-3 pb-2 lg:grid-flow-col lg:auto-cols-[minmax(11rem,1fr)] lg:items-start lg:overflow-x-auto">
				{COLUMNS.map((column) => {
					const items = cards.filter((card) => card.column === column)
					return (
						<BoardColumn
							key={column}
							column={column}
							items={items}
							lit={
								overColumn === column &&
								active !== undefined &&
								canDrop(active.column, column)
							}
						>
							<SortableContext
								items={items.map((card) => card.id)}
								strategy={noShift}
							>
								{items.map((card) => (
									<DraggableCard
										key={card.id}
										card={card}
										isPending={pendingCardId === card.id}
										expanded={expandedIds.has(card.id)}
										onToggleExpanded={onToggleExpanded}
										onMove={onMove}
										onExecute={onExecute}
										onOpen={onOpen}
									/>
								))}
							</SortableContext>
						</BoardColumn>
					)
				})}
			</div>
			<DragOverlay>
				{active ? (
					<CardItem
						card={active}
						isPending={false}
						expanded={false}
						onToggleExpanded={noop}
						onMove={noop}
						dragging="overlay"
					/>
				) : null}
			</DragOverlay>
		</DndContext>
	)
}

function noop() {}
