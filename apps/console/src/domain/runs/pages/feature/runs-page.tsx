import { RunsView } from '#/domain/runs/pages/ui/runs-view'

export function RunsPage() {
	return (
		<RunsView
			heading="Runs"
			emptyTitle="Aucun run"
			emptyDescription="Les runs d'agents apparaîtront ici dès leur lancement."
		/>
	)
}
