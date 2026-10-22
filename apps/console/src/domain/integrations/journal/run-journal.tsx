import type { Schemas } from '#/api/api.client'
import { JournalSection } from '#/domain/integrations/journal/journal-section'
import { useRunIntegrationActions } from '#/domain/integrations/journal/use-integration-actions'
import { isInterventionRun } from '#/domain/runs/run-title'

interface Props {
	run: Schemas.RunResponse
}

export function RunJournal({ run }: Props) {
	const intervention = isInterventionRun(run)
	const polling = run.status === 'queued' || run.status === 'running'
	const journal = useRunIntegrationActions(
		intervention ? run.id : undefined,
		polling,
	)
	if (!intervention) return null

	return (
		<JournalSection
			items={journal.data?.items}
			isError={journal.isError}
			headingLevel="h2"
			locked={polling}
		/>
	)
}
