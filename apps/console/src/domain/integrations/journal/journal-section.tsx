import type { Schemas } from '#/api/api.client'
import { InlineAlert } from '#/components/layout/inline-alert'
import { SectionCard } from '#/components/layout/section-card'
import { IntegrationJournal } from '#/domain/integrations/journal/integration-journal'

export const JOURNAL_TITLE = 'Journal des intégrations'

interface Props {
	items: ReadonlyArray<Schemas.IntegrationActionResponse> | undefined
	isError: boolean
	headingLevel: 'h2' | 'h3'
	locked: boolean
}

export function JournalSection({
	items,
	isError,
	headingLevel,
	locked,
}: Props) {
	if (isError) {
		return (
			<SectionCard title={JOURNAL_TITLE} headingLevel={headingLevel}>
				<InlineAlert tone="danger">
					Impossible de charger le journal des intégrations.
				</InlineAlert>
			</SectionCard>
		)
	}
	if (!items || items.length === 0) return null

	return (
		<SectionCard title={JOURNAL_TITLE} headingLevel={headingLevel} flush>
			<IntegrationJournal actions={items} revert={{ locked }} />
		</SectionCard>
	)
}
