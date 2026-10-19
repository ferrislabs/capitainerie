import type { ReactNode } from 'react'
import { Badge } from '#/components/ui/badge'
import type { MessageView } from '#/domain/chat/chat-state'
import { ToolCalls } from '#/domain/chat/ui/tool-calls'
import { cn } from '#/lib/utils'

interface Props {
	messages: ReadonlyArray<MessageView>
	renderCard: (proposal: MessageView['cards'][number]) => ReactNode
	pendingUserText?: string
	live?: ReactNode
}

function Body({ text }: { text: string }) {
	return (
		<p className="text-sm leading-relaxed break-words whitespace-pre-wrap">
			{text}
		</p>
	)
}

export function AssistantMessage({
	message,
	renderCard,
}: {
	message: MessageView
	renderCard: Props['renderCard']
}) {
	return (
		<div className="flex flex-col gap-2">
			<ToolCalls tools={message.tools} />
			{message.text ? <Body text={message.text} /> : null}
			{message.cards.length > 0 ? (
				<ul aria-label="Cartes proposées" className="flex flex-col gap-2">
					{message.cards.map((proposal) => (
						<li key={proposal.cardId}>{renderCard(proposal)}</li>
					))}
				</ul>
			) : null}
			{message.status === 'interrupted' ? (
				<Badge variant="neutral">Réponse interrompue</Badge>
			) : null}
			{message.status === 'failed' ? (
				<Badge variant="danger">Le Capitaine n’a pas pu répondre</Badge>
			) : null}
		</div>
	)
}

export function MessageList({
	messages,
	renderCard,
	pendingUserText,
	live,
}: Props) {
	return (
		<ol
			aria-label="Messages"
			aria-live="polite"
			className="flex flex-col gap-5"
		>
			{messages.map((message) => (
				<li
					key={message.id}
					data-role={message.role}
					className={cn(message.role === 'user' && 'flex justify-end')}
				>
					{message.role === 'user' ? (
						<div className="max-w-[85%] rounded-lg bg-muted px-3 py-2">
							<Body text={message.text} />
						</div>
					) : (
						<AssistantMessage message={message} renderCard={renderCard} />
					)}
				</li>
			))}
			{pendingUserText ? (
				<li data-role="user" className="flex justify-end">
					<div className="max-w-[85%] rounded-lg bg-muted px-3 py-2">
						<Body text={pendingUserText} />
					</div>
				</li>
			) : null}
			{live ? (
				<li data-role="assistant" aria-busy="true">
					{live}
				</li>
			) : null}
		</ol>
	)
}
