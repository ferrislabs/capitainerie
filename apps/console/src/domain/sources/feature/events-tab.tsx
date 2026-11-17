import { Inbox } from 'lucide-react'
import { EmptyState } from '#/components/layout/empty-state'
import { SectionCard } from '#/components/layout/section-card'
import { eventsFor } from '#/domain/sources/mock/events'
import { EventsTable } from '#/domain/sources/ui/events-table'

interface Props {
	sourceId: string
}

export function EventsTab({ sourceId }: Props) {
	const events = eventsFor(sourceId)
	return (
		<SectionCard title="Derniers événements" flush>
			{events.length > 0 ? (
				<EventsTable events={events} />
			) : (
				<EmptyState
					icon={Inbox}
					title="Aucun événement pour l'instant"
					description="Utilise « Envoyer un test » pour vérifier la chaîne."
				/>
			)}
		</SectionCard>
	)
}
