import { useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { useClients, useProjects } from '#/domain/clients/hooks/use-directory'
import { useMe } from '#/domain/identity/hooks/use-me'
import type { Column } from '#/domain/kanban/columns'
import { executionPullRequest } from '#/domain/kanban/feature/execution-pull-request'
import { useCardDetail } from '#/domain/kanban/hooks/use-card-detail'
import { useCommentCard } from '#/domain/kanban/hooks/use-comment-card'
import { usePatchCard } from '#/domain/kanban/hooks/use-patch-card'
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
	const patch = usePatchCard()
	const open = cardId !== undefined
	const clients = useClients({ enabled: open })
	const projects = useProjects({ enabled: open })
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
			open={open}
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
			clients={clients.data?.items.map((item) => item.client) ?? []}
			projects={projects.data?.items.map((item) => item.project) ?? []}
			isLinking={patch.isPending}
			linksReady={clients.isSuccess && projects.isSuccess}
			linkError={patch.errorMessage}
			onLink={(links) => {
				if (!card) return
				patch.mutate({
					path: { card_id: card.id },
					body: { expected_version: card.version, ...links },
				})
			}}
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
