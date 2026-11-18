import { Link } from '@tanstack/react-router'
import { Maximize2 } from 'lucide-react'
import { InlineAlert } from '#/components/layout/inline-alert'
import {
	Sheet,
	SheetBody,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
} from '#/components/ui/sheet'
import { useArtifact } from '#/domain/artifacts/hooks/use-artifacts'
import { type Artifact, kindLabel } from '#/domain/artifacts/model'
import { ArtifactContent } from '#/domain/artifacts/ui/artifact-content'
import { relativeAge } from '#/domain/dashboard/format'

interface Props {
	artifact: Artifact | null
	onClose: () => void
}

function Content({ artifact }: { artifact: Artifact }) {
	const detail = useArtifact(artifact.id)
	if (detail.isPending) {
		return (
			<output className="block text-[13px] text-muted-foreground">
				Chargement…
			</output>
		)
	}
	if (detail.isError) {
		return (
			<InlineAlert tone="danger">
				Impossible de charger le contenu de l'artifact.
			</InlineAlert>
		)
	}
	return <ArtifactContent artifact={detail.data} />
}

export function ArtifactSheet({ artifact, onClose }: Props) {
	return (
		<Sheet open={artifact !== null} onOpenChange={(open) => !open && onClose()}>
			<SheetContent className="sm:max-w-[860px]">
				{artifact ? (
					<>
						<SheetHeader>
							<SheetTitle>{artifact.title}</SheetTitle>
							<SheetDescription>
								{kindLabel(artifact.kind)}
								{artifact.agent_name ? ` · ${artifact.agent_name}` : ''} · mis à
								jour {relativeAge(artifact.updated_at)}
							</SheetDescription>
							<div className="flex min-w-0 flex-wrap items-center gap-x-4 gap-y-1 text-xs">
								<Link
									to="/artifacts/$artifactId"
									params={{ artifactId: artifact.id }}
									className="inline-flex items-center gap-1 font-medium text-primary hover:underline"
								>
									<Maximize2 aria-hidden="true" className="size-3.5" />
									Ouvrir en pleine page
								</Link>
								{artifact.card_id ? (
									<Link
										to="/tasks/$taskId"
										params={{ taskId: artifact.card_id }}
										className="min-w-0 truncate text-primary hover:underline"
									>
										{artifact.card_title ?? 'Voir la tâche'}
									</Link>
								) : null}
							</div>
						</SheetHeader>
						<SheetBody className="px-6 py-6 sm:px-8">
							<Content artifact={artifact} />
						</SheetBody>
					</>
				) : null}
			</SheetContent>
		</Sheet>
	)
}
