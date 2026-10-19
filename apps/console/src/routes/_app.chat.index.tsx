import { createFileRoute } from '@tanstack/react-router'
import { ConversationsPage } from '#/domain/chat/feature/conversations-page'

export const Route = createFileRoute('/_app/chat/')({
	component: ConversationsPage,
})
