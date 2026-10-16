import { AgentsView } from '#/domain/agents/pages/ui/agents-view'

export function AgentsPage() {
	return (
		<AgentsView
			heading="Agents"
			emptyTitle="Aucun agent"
			emptyDescription="Les agents configurés apparaîtront ici."
		/>
	)
}
