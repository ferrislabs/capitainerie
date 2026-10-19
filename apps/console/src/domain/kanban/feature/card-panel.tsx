import { useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { useMe } from '#/domain/identity/hooks/use-me'
import type { Column } from '#/domain/kanban/columns'
import { executionPullRequest } from '#/domain/kanban/feature/execution-pull-request'
import { useCardDetail } from '#/domain/kanban/hooks/use-card-detail'
import { useCommentCard } from '#/domain/kanban/hooks/use-comment-card'
import { CardSheet, type CardSheetStatus } from '#/domain/kanban/ui/card-sheet'
import { isApiRequestError } from '#/lib/api-error'

interface Props {
	cardId: string | undefined
	listedCard: Schemas.CardResponse | undefined
	isMoving: boolean
	moveError: string | undefined
	onClose: () => void
	onMove: (cardId: string, to: Column, expectedVersion: number) => void
	onExecute: (card: Schemas.CardResponse) => void
}

export function CardPanel({
	cardId,
	listedCard,
	isMoving,
	moveError,
	onClose,
	onMove,
	onExecute,
}: Props) {
	const detail = useCardDetail(cardId)
	const me = useMe()
	const comment = useCommentCard()
	const [draft, setDraft] = useState('')

	const card = listedCard ?? detail.data?.card

	function status(): CardSheetStatus {
		if (detail.data) return 'ready'
		if (isApiRequestError(detail.error) && detail.error.status === 404) {
			return 'not_found'
		}
		if (detail.isError) return 'error'
		return 'loading'
	}

	function submitComment() {
		if (!cardId) return
		comment.mutate(
			{ path: { card_id: cardId }, body: { body: draft } },
			{ onSuccess: () => setDraft('') },
		)
	}

	return (
		<CardSheet
			open={cardId !== undefined}
			status={status()}
			card={card}
			comments={detail.data?.comments ?? []}
			runs={detail.data?.runs ?? []}
			currentSubject={me.data?.subject}
			isMoving={isMoving}
			moveError={moveError}
			draft={draft}
			isCommenting={comment.isPending}
			commentError={comment.errorMessage}
			runExtra={executionPullRequest}
			onClose={onClose}
			onMove={(to) => {
				if (card) onMove(card.id, to, card.version)
			}}
			onExecute={onExecute}
			onDraftChange={(value) => {
				comment.reset()
				setDraft(value)
			}}
			onSubmitComment={submitComment}
		/>
	)
}
