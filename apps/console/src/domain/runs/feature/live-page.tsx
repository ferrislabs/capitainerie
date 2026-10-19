import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { TableSkeleton } from '#/components/layout/table-skeleton'
import { useActiveRuns } from '#/domain/runs/hooks/use-active-runs'
import { ActiveRunsTable } from '#/domain/runs/ui/active-runs-table'
import { useNow } from '#/hooks/use-now'

export function LivePage() {
	const runs = useActiveRuns()
	const items = runs.data?.items
	const nowMs = useNow(items !== undefined && items.length > 0)

	return (
		<Page>
			<PageHeader
				title="En cours"
				description="Les agents qui tournent en ce moment, mis à jour en direct."
			/>
			<SectionCard
				title="Runs actifs"
				description={
					items === undefined
						? undefined
						: `${items.length} run${items.length > 1 ? 's' : ''} en attente ou en cours.`
				}
				bare
			>
				{runs.isError ? (
					<InlineAlert tone="danger">
						Impossible de charger les runs en cours.
					</InlineAlert>
				) : runs.isPending ? (
					<TableSkeleton rows={4} columns={6} />
				) : (
					<ActiveRunsTable runs={runs.data.items} nowMs={nowMs} />
				)}
			</SectionCard>
		</Page>
	)
}
