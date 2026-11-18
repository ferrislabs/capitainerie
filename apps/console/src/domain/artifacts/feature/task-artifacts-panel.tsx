import { useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { ArtifactSheet } from '#/domain/artifacts/feature/artifact-sheet'
import { useCardArtifacts } from '#/domain/artifacts/hooks/use-artifacts'
import { type Artifact, kindLabel } from '#/domain/artifacts/model'
import { KIND_ICON } from '#/domain/artifacts/ui/artifact-card'
import { relativeAge } from '#/domain/dashboard/format'

/** Artifacts produits pour une tâche ; rien n'est affiché tant qu'il n'y en a pas. */
export function TaskArtifactsPanel({ cardId }: { cardId: string }) {
	const query = useCardArtifacts(cardId)
	const [open, setOpen] = useState<Artifact | null>(null)
	const items = query.data?.items ?? []
	if (items.length === 0) return null
	return (
		<>
			<SectionCard
				title="Artifacts"
				description={String(items.length)}
				flush
				className="my-2"
			>
				<ul className="m-0 list-none p-0">
					{items.map((artifact) => {
						const Icon = KIND_ICON[artifact.kind]
						return (
							<li key={artifact.id}>
								<button
									type="button"
									onClick={() => setOpen(artifact)}
									className="flex min-h-9 w-full min-w-0 items-center gap-2.5 border-t px-3 py-1.5 text-left first:border-t-0 hover:bg-secondary focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring"
								>
									<Icon
										aria-hidden="true"
										className="size-4 shrink-0 text-muted-foreground"
									/>
									<span className="min-w-0 flex-1 truncate font-medium">
										{artifact.title}
									</span>
									<span className="text-xs text-muted-foreground">
										{kindLabel(artifact.kind)}
									</span>
									<span className="font-mono text-[11.5px] whitespace-nowrap text-faint">
										{relativeAge(artifact.updated_at)}
									</span>
								</button>
							</li>
						)
					})}
				</ul>
			</SectionCard>
			<ArtifactSheet artifact={open} onClose={() => setOpen(null)} />
		</>
	)
}
