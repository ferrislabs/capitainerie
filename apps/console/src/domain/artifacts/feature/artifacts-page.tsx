import { Box } from 'lucide-react'
import { useState } from 'react'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import { ArtifactSheet } from '#/domain/artifacts/feature/artifact-sheet'
import { useArtifacts } from '#/domain/artifacts/hooks/use-artifacts'
import {
	ARTIFACT_KINDS,
	type Artifact,
	type ArtifactKind,
} from '#/domain/artifacts/model'
import { ArtifactCard } from '#/domain/artifacts/ui/artifact-card'
import { useAgentHue } from '#/domain/audit/hooks/use-agent-hue'

export function ArtifactsPage() {
	const [kind, setKind] = useState<ArtifactKind | ''>('')
	const [agentId, setAgentId] = useState('')
	const [open, setOpen] = useState<Artifact | null>(null)
	const agents = useTeamAgents().data?.items ?? []
	const hueOf = useAgentHue()
	const query = useArtifacts({
		...(kind ? { kind } : {}),
		...(agentId ? { agent_id: agentId } : {}),
	})
	const artifacts = query.data?.pages.flatMap((page) => page.items) ?? []
	const filtered = kind !== '' || agentId !== ''

	return (
		<Page className="leading-[1.4]">
			<PageHeader
				title="Artifacts"
				description="Livrables produits par les agents"
			/>
			<div className="mb-2.5 flex flex-wrap gap-2">
				<NativeSelect
					aria-label="Filtrer par type"
					value={kind}
					onChange={(e) => setKind(e.target.value as ArtifactKind | '')}
					className="w-auto"
				>
					<option value="">Tous les types</option>
					{ARTIFACT_KINDS.map((item) => (
						<option key={item.kind} value={item.kind}>
							{item.label}
						</option>
					))}
				</NativeSelect>
				<NativeSelect
					aria-label="Filtrer par agent"
					value={agentId}
					onChange={(e) => setAgentId(e.target.value)}
					className="w-auto"
				>
					<option value="">Tous les agents</option>
					{agents.map((agent) => (
						<option key={agent.id} value={agent.id}>
							{agent.name}
						</option>
					))}
				</NativeSelect>
			</div>
			{query.isPending ? (
				<output className="block text-[13px] text-muted-foreground">
					Chargement…
				</output>
			) : query.isError ? (
				<InlineAlert tone="danger">
					Impossible de charger les artifacts.
				</InlineAlert>
			) : artifacts.length === 0 ? (
				<EmptyState
					icon={Box}
					title={
						filtered ? 'Aucun artifact pour ces filtres' : 'Aucun artifact'
					}
					description="Les documents, données et liens produits par les agents apparaîtront ici."
				/>
			) : (
				<>
					<div className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-2.5">
						{artifacts.map((artifact) => (
							<ArtifactCard
								key={artifact.id}
								artifact={artifact}
								hue={hueOf(artifact.agent_id, artifact.agent_name ?? '')}
								onOpen={setOpen}
							/>
						))}
					</div>
					{query.hasNextPage ? (
						<div className="mt-2.5 flex justify-center">
							<Button
								variant="outline"
								disabled={query.isFetchingNextPage}
								onClick={() => void query.fetchNextPage()}
							>
								{query.isFetchingNextPage ? 'Chargement…' : 'Charger plus'}
							</Button>
						</div>
					) : null}
				</>
			)}
			<ArtifactSheet artifact={open} onClose={() => setOpen(null)} />
		</Page>
	)
}
