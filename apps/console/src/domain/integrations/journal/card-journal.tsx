import type { Schemas } from '#/api/api.client'
import { JournalSection } from '#/domain/integrations/journal/journal-section'
import { useCardIntegrationActions } from '#/domain/integrations/journal/use-integration-actions'
import { isInterventionRun } from '#/domain/runs/run-title'

interface Props {
	cardId: string
	runs: ReadonlyArray<Schemas.RunResponse>
}

export function CardJournal({ cardId, runs }: Props) {
	const interventions = runs.filter(isInterventionRun)
	const polling = interventions.some(
		(run) => run.status === 'queued' || run.status === 'running',
	)
	const journal = useCardIntegrationActions(
		interventions.length > 0 ? cardId : undefined,
		polling,
	)
	if (interventions.length === 0) return null

	return (
		<JournalSection
			items={journal.data?.items}
			isError={journal.isError}
			headingLevel="h3"
			locked={polling}
		/>
	)
}
