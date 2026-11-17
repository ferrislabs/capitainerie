import { FileText, Search } from 'lucide-react'
import { useState } from 'react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { EmptyState } from '#/components/layout/empty-state'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { TextInput } from '#/components/ui/text-input'
import { filterArtifacts } from '#/domain/artifacts/filter-artifacts'
import { ARTIFACTS } from '#/domain/artifacts/mock/artifacts'
import { ArtifactsTable } from '#/domain/artifacts/ui/artifacts-table'

export function ArtifactsPage() {
	const [query, setQuery] = useState('')
	const artifacts = filterArtifacts(query, ARTIFACTS)
	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Artifacts"
				description="Livrables produits par les agents"
			/>
			<TextInput
				type="search"
				aria-label="Rechercher un artifact"
				placeholder="Rechercher par nom, agent ou type"
				leading={<Search aria-hidden="true" className="size-4" />}
				value={query}
				onChange={(event) => setQuery(event.target.value)}
				wrapperClassName="max-w-sm"
			/>
			{artifacts.length === 0 ? (
				<EmptyState
					icon={FileText}
					title="Aucun artifact"
					description="Aucun livrable ne correspond à cette recherche."
				/>
			) : (
				<ArtifactsTable artifacts={artifacts} />
			)}
		</Page>
	)
}
