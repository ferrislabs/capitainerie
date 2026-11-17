import { Filter } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { ARTIFACTS } from '#/domain/artifacts/mock/artifacts'
import { ArtifactCard } from '#/domain/artifacts/ui/artifact-card'
import { Toast } from '#/domain/tasks/ui/toast'

export function ArtifactsPage() {
	const [message, notify] = useNotice()
	return (
		<Page className="leading-[1.4]">
			<DemoBanner />
			<PageHeader
				title="Artifacts"
				description="Livrables produits par les agents"
				actions={
					<Button
						variant="outline"
						onClick={() => notify('Filtre par type et par agent')}
					>
						<Filter aria-hidden="true" />
						Filtrer
					</Button>
				}
			/>
			<div className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-2.5">
				{ARTIFACTS.map((artifact) => (
					<ArtifactCard
						key={artifact.id}
						artifact={artifact}
						onOpen={(a) => notify(`Ouverture de « ${a.name} »`)}
					/>
				))}
			</div>
			<Toast message={message} />
		</Page>
	)
}
