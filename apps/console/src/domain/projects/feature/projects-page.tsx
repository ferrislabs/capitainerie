import { DemoBanner } from '#/components/layout/demo-banner'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'

export function ProjectsPage() {
	return (
		<Page>
			<DemoBanner />
			<PageHeader title="Projets" />
		</Page>
	)
}
