import { createFileRoute } from '@tanstack/react-router'
import { ChatPage } from '#/domain/chat/feature/chat-page'

export const Route = createFileRoute('/_app/chat/$id')({
	component: ChatRoute,
})

function ChatRoute() {
	const { id } = Route.useParams()
	return <ChatPage key={id} conversationId={id} />
}
