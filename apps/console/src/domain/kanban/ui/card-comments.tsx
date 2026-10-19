import type { FormEvent } from 'react'
import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { Label } from '#/components/ui/label'
import { Textarea } from '#/components/ui/textarea'
import { agentName } from '#/domain/kanban/columns'
import { relativeDate } from '#/domain/kanban/relative-date'

export const COMMENT_MAX_CHARS = 10_000

interface Props {
	comments: ReadonlyArray<Schemas.CommentResponse>
	currentSubject: string | undefined
	draft: string
	isPending: boolean
	errorMessage: string | undefined
	onDraftChange: (value: string) => void
	onSubmit: () => void
}

export function commentLength(draft: string): number {
	return [...draft].length
}

export function commentCanBeSent(draft: string): boolean {
	return draft.trim() !== '' && commentLength(draft) <= COMMENT_MAX_CHARS
}

function Author({
	comment,
	currentSubject,
}: {
	comment: Schemas.CommentResponse
	currentSubject: string | undefined
}) {
	if (comment.author_actor) {
		return (
			<Badge variant="secondary" className="font-mono text-[11px]">
				{agentName(comment.author_actor)}
			</Badge>
		)
	}
	return (
		<span className="text-xs font-medium">
			{comment.author_subject === currentSubject ? 'Vous' : 'Un membre'}
		</span>
	)
}

export function CardComments({
	comments,
	currentSubject,
	draft,
	isPending,
	errorMessage,
	onDraftChange,
	onSubmit,
}: Props) {
	function handleSubmit(event: FormEvent<HTMLFormElement>) {
		event.preventDefault()
		if (commentCanBeSent(draft) && !isPending) onSubmit()
	}

	return (
		<section
			aria-labelledby="card-comments-title"
			className="flex flex-col gap-3"
		>
			<h3 id="card-comments-title" className="text-[15px] font-semibold">
				Commentaires
			</h3>
			{comments.length === 0 ? (
				<p className="text-[13px] text-muted-foreground">Aucun commentaire.</p>
			) : (
				<ol className="flex flex-col gap-3">
					{comments.map((comment) => (
						<li
							key={comment.id}
							className="flex flex-col gap-1 rounded-md border bg-card p-3"
						>
							<div className="flex items-center justify-between gap-2">
								<Author comment={comment} currentSubject={currentSubject} />
								<time
									dateTime={comment.created_at}
									className="text-[11px] text-muted-foreground"
								>
									{relativeDate(comment.created_at)}
								</time>
							</div>
							<p className="text-[13px] leading-relaxed break-words whitespace-pre-wrap">
								{comment.body}
							</p>
						</li>
					))}
				</ol>
			)}
			<form onSubmit={handleSubmit} className="flex flex-col gap-2">
				<Label htmlFor="card-comment-body">Ajouter un commentaire</Label>
				<Textarea
					id="card-comment-body"
					value={draft}
					disabled={isPending}
					onChange={(event) => onDraftChange(event.target.value)}
				/>
				{errorMessage ? (
					<InlineAlert tone="danger">{errorMessage}</InlineAlert>
				) : null}
				<div className="flex items-center justify-between gap-2">
					<span className="text-xs text-muted-foreground figures">
						{commentLength(draft)} / {COMMENT_MAX_CHARS}
					</span>
					<Button
						type="submit"
						size="sm"
						disabled={isPending || !commentCanBeSent(draft)}
					>
						Commenter
					</Button>
				</div>
			</form>
		</section>
	)
}
