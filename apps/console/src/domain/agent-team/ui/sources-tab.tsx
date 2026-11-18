import { Cable } from 'lucide-react'
import { SectionCard } from '#/components/layout/section-card'
import { Empty } from '#/domain/agent-team/ui/rows'

export function SourcesTab() {
	return (
		<SectionCard title="Sources écoutées" flush>
			<Empty
				icon={<Cable aria-hidden="true" className="mx-auto size-4" />}
				title="Aucune source branchée"
			>
				Cet agent se réveille sur sa cadence ou quand une tâche lui est
				assignée. Le branchement de sources sur un agent n'est pas encore
				disponible.
			</Empty>
		</SectionCard>
	)
}
