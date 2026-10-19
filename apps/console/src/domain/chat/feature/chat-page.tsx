import { Link, useNavigate } from '@tanstack/react-router'
import { ArrowLeft } from 'lucide-react'
import { useCallback, useEffect, useRef, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Skeleton } from '#/components/ui/skeleton'
import {
	CONVERSATION_FULL_MESSAGE,
	chatErrorMessage,
} from '#/domain/chat/chat-errors'
import {
	type MessageView,
	messageViews,
	needsAnswer,
} from '#/domain/chat/chat-state'
import { useCaptainModel } from '#/domain/chat/hooks/use-captain-model'
import { useChatTurn } from '#/domain/chat/hooks/use-chat-turn'
import { useConversation } from '#/domain/chat/hooks/use-conversation'
import { appendDictation } from '#/domain/chat/hooks/use-new-conversation'
import { useVoiceInput } from '#/domain/chat/hooks/use-voice-input'
import { Composer } from '#/domain/chat/ui/composer'
import { AssistantMessage, MessageList } from '#/domain/chat/ui/message-list'
import { ProposalCard } from '#/domain/chat/ui/proposal-card'
import { ExecuteCardFlow } from '#/domain/kanban/feature/execute-card-flow'
import { useCards } from '#/domain/kanban/hooks/use-cards'
import { useMoveCard } from '#/domain/kanban/hooks/use-move-card'
import { formatTokens } from '#/domain/runs/format'

interface Props {
	conversationId: string
}

function Thinking() {
	return (
		<output className="block text-sm text-muted-foreground">
			Le Capitaine réfléchit…
		</output>
	)
}

export function ChatPage({ conversationId }: Props) {
	const navigate = useNavigate()
	const conversation = useConversation(conversationId)
	const cards = useCards()
	const move = useMoveCard()
	const captain = useCaptainModel()
	const turn = useChatTurn(conversationId)
	const [value, setValue] = useState('')
	const [executingId, setExecutingId] = useState<string>()
	const started = useRef<string | undefined>(undefined)
	const send = turn.send
	const abort = turn.abort
	const streaming = turn.state.status === 'streaming'
	const data = conversation.data
	const lastId = data?.messages.at(-1)?.id

	const voice = useVoiceInput(
		useCallback(
			(text: string) => setValue((current) => appendDictation(current, text)),
			[],
		),
	)

	useEffect(() => {
		if (!data || streaming || lastId === undefined) return
		if (!needsAnswer(data.messages) || started.current === lastId) return
		started.current = lastId
		void send()
	}, [data, streaming, lastId, send])

	useEffect(() => {
		if (!streaming) return
		function onKeyDown(event: KeyboardEvent) {
			if (event.key === 'Escape') abort()
		}
		window.addEventListener('keydown', onKeyDown)
		return () => window.removeEventListener('keydown', onKeyDown)
	}, [streaming, abort])

	function submit() {
		const message = value.trim()
		if (message === '' || streaming) return
		setValue('')
		void send(message)
	}

	if (conversation.isError) {
		return (
			<Page>
				<InlineAlert tone="danger">
					{chatErrorMessage(conversation.error)}
				</InlineAlert>
			</Page>
		)
	}

	if (!data) {
		return (
			<Page>
				<Skeleton className="h-8 w-64" />
				<Skeleton className="h-24 w-full" />
				<span className="sr-only">Chargement…</span>
			</Page>
		)
	}

	const messages = messageViews(data.messages)
	const cardsById = new Map(cards.data?.items.map((card) => [card.id, card]))
	const { full, model, input_tokens, output_tokens } = data.conversation
	const live: MessageView | undefined = streaming
		? {
				id: 'live',
				role: 'assistant',
				text: turn.state.draft.text,
				status: 'complete',
				tools: turn.state.draft.tools,
				cards: turn.state.draft.cards,
			}
		: undefined

	function renderCard(proposal: MessageView['cards'][number]) {
		const card = cardsById.get(proposal.cardId)
		return (
			<ProposalCard
				proposal={proposal}
				card={card}
				pending={move.pendingCardId === proposal.cardId}
				errorMessage={
					move.variables?.path.card_id === proposal.cardId
						? move.errorMessage
						: undefined
				}
				onValidate={(target) =>
					move.mutate({
						path: { card_id: target.id },
						body: { to: 'todo', expected_version: target.version },
					})
				}
				onReject={(target) =>
					move.mutate({
						path: { card_id: target.id },
						body: { to: 'rejected', expected_version: target.version },
					})
				}
				onExecute={(target) => {
					if (target.column !== 'proposed') {
						setExecutingId(target.id)
						return
					}
					move.mutate(
						{
							path: { card_id: target.id },
							body: { to: 'todo', expected_version: target.version },
						},
						{ onSuccess: () => setExecutingId(target.id) },
					)
				}}
			/>
		)
	}

	return (
		<Page className="min-h-full">
			<PageHeader
				leading={
					<Link
						to="/chat"
						className="inline-flex w-fit items-center gap-1.5 text-[13px] text-muted-foreground hover:text-foreground"
					>
						<ArrowLeft aria-hidden="true" className="size-3.5" />
						Conversations
					</Link>
				}
				title={data.conversation.title}
				description={
					<span className="figures">
						{model ? `${model} · ` : ''}
						{formatTokens(input_tokens + output_tokens)} tokens
					</span>
				}
			/>
			<MessageList
				messages={messages}
				renderCard={renderCard}
				pendingUserText={streaming ? turn.state.pendingMessage : undefined}
				live={
					live ? (
						live.text === '' &&
						live.tools.length === 0 &&
						live.cards.length === 0 ? (
							<Thinking />
						) : (
							<AssistantMessage message={live} renderCard={renderCard} />
						)
					) : undefined
				}
			/>
			{turn.state.error ? (
				<InlineAlert tone="danger">{turn.state.error}</InlineAlert>
			) : null}
			{full ? (
				<InlineAlert tone="info">
					{CONVERSATION_FULL_MESSAGE}{' '}
					<Link to="/" className="underline underline-offset-4">
						Nouvelle conversation
					</Link>
				</InlineAlert>
			) : null}
			<Composer
				className="sticky bottom-0 bg-background pb-4"
				label="Message au Capitaine"
				placeholder="Répondre au Capitaine…"
				value={value}
				onChange={setValue}
				onSend={submit}
				onAbort={abort}
				streaming={streaming}
				disabled={full}
				voice={{
					state: voice.state,
					supported: voice.supported,
					onToggle: voice.toggle,
					onCancel: voice.cancel,
				}}
				model={captain}
			/>
			{voice.error ? (
				<InlineAlert tone="danger">{voice.error}</InlineAlert>
			) : null}
			<ExecuteCardFlow
				card={cards.data?.items.find((card) => card.id === executingId)}
				onClose={() => setExecutingId(undefined)}
				onStarted={(runId) => {
					setExecutingId(undefined)
					void navigate({ to: '/runs/$runId', params: { runId } })
				}}
			/>
		</Page>
	)
}
