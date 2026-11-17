import { SectionCard } from '#/components/layout/section-card'
import { eventsFor } from '#/domain/sources/mock/events'
import { EventsTable } from '#/domain/sources/ui/events-table'
import { Empty } from '#/domain/sources/ui/mockup'

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
				<Empty title="Aucun événement pour l'instant">
					Utilise « Envoyer un test » pour vérifier la chaîne.
				</Empty>
			)}
		</SectionCard>
	)
}
