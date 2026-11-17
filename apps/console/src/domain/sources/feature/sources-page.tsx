import { Link } from '@tanstack/react-router'
import { Plus } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { updateSource, useDemoState } from '#/domain/demo'
import { SourcesTable } from '#/domain/sources/ui/sources-table'

export function SourcesPage() {
	const sources = useDemoState((s) => s.sources)
	const agents = useDemoState((s) => s.agents)
	const active = sources.filter((s) => s.enabled).length
	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Sources"
				description={`Ce que tes agents écoutent · ${sources.length} sources · ${active} actives`}
				actions={
					<Button asChild>
						<Link to="/sources/nouvelle">
							<Plus aria-hidden="true" />
							Nouvelle source
						</Link>
					</Button>
				}
			/>
			<div className="overflow-hidden rounded-lg border bg-card">
				<SourcesTable
					sources={sources}
					agents={agents}
					onToggle={(source, enabled) => updateSource(source.id, { enabled })}
				/>
			</div>
		</Page>
	)
}
