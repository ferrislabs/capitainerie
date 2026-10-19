import { useSortable } from '@dnd-kit/sortable'
import type { KeyboardEvent, MouseEvent } from 'react'
import type { Schemas } from '#/api/api.client'
import type { Column } from '#/domain/kanban/columns'
import { CardItem } from '#/domain/kanban/ui/card-item'

interface Props {
	card: Schemas.CardResponse
	isPending: boolean
	expanded: boolean
	onToggleExpanded: (cardId: string) => void
	onMove: (cardId: string, to: Column, expectedVersion: number) => void
	onExecute?: (card: Schemas.CardResponse) => void
	onOpen: (cardId: string) => void
}

type KeyHandler = (event: KeyboardEvent<HTMLElement>) => void

const INTERACTIVE = 'button, a, select, input, textarea'

export function DraggableCard({
	card,
	isPending,
	expanded,
	onToggleExpanded,
	onMove,
	onExecute,
	onOpen,
}: Props) {
	const { attributes, listeners, setNodeRef, isDragging } = useSortable({
		id: card.id,
		data: { column: card.column },
		disabled: isPending,
		attributes: { role: 'group', roleDescription: 'carte déplaçable' },
	})
	const dragKeyDown = listeners?.onKeyDown as KeyHandler | undefined

	function handleKeyDown(event: KeyboardEvent<HTMLElement>) {
		dragKeyDown?.(event)
		if (event.key === 'Enter' && event.target === event.currentTarget) {
			event.preventDefault()
			onOpen(card.id)
		}
	}

	function handleClick(event: MouseEvent<HTMLElement>) {
		if (event.target instanceof Element && event.target.closest(INTERACTIVE)) {
			return
		}
		onOpen(card.id)
	}

	const wrapper = {
		...attributes,
		...listeners,
		'aria-label': card.title,
		onKeyDown: handleKeyDown,
		onClick: handleClick,
	}

	return (
		<div
			ref={setNodeRef}
			{...wrapper}
			data-card-id={card.id}
			className="cursor-grab rounded-md outline-none focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
		>
			<CardItem
				card={card}
				isPending={isPending}
				expanded={expanded}
				onToggleExpanded={onToggleExpanded}
				onMove={onMove}
				onExecute={onExecute}
				dragging={isDragging ? 'origin' : undefined}
			/>
		</div>
	)
}
