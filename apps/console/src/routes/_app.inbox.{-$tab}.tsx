import { createFileRoute } from '@tanstack/react-router'
import { InboxPage } from '#/domain/inbox/feature/inbox-page'
import { parseInboxTab } from '#/domain/inbox/tabs'

export const Route = createFileRoute('/_app/inbox/{-$tab}')({
	component: InboxRoute,
})

function InboxRoute() {
	const { tab } = Route.useParams()
	return <InboxPage tab={parseInboxTab(tab)} />
}
