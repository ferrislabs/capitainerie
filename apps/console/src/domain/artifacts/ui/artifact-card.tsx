import { Link } from '@tanstack/react-router'
import { Braces, FileText, Link2, Maximize2 } from 'lucide-react'
import {
	type Artifact,
	type ArtifactKind,
	kindLabel,
} from '#/domain/artifacts/model'
import { relativeAge } from '#/domain/dashboard/format'
import { AgentChip } from '#/domain/tasks/ui/agent-chip'
import { Sev } from '#/domain/tasks/ui/sev'
import { shortName } from '#/lib/agent-name'

interface Props {
	artifact: Artifact
	hue: number
	onOpen: (artifact: Artifact) => void
}

export const KIND_ICON: Readonly<Record<ArtifactKind, typeof FileText>> = {
	markdown: FileText,
	json: Braces,
	link: Link2,
}

export function ArtifactCard({ artifact, hue, onOpen }: Props) {
	const Icon = KIND_ICON[artifact.kind]
	return (
		<article className="relative min-w-0 overflow-hidden rounded-lg border bg-card">
			<button
				type="button"
				onClick={() => onOpen(artifact)}
				className="block w-full cursor-pointer text-left focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-ring"
			>
				<div className="grid h-[92px] place-items-center rounded-t-lg border-b bg-[repeating-linear-gradient(135deg,var(--secondary)_0_8px,var(--background)_8px_16px)] text-faint">
					<Icon aria-hidden="true" className="size-4" />
				</div>
				<div className="flex flex-col gap-[5px] px-3 py-[11px]">
					<h3 className="m-0 truncate text-[13px] font-semibold">
						{artifact.title}
					</h3>
					<div className="flex justify-between gap-2">
						{artifact.agent_name ? (
							<AgentChip
								name={shortName(artifact.agent_name)}
								hue={hue}
								size={16}
							/>
						) : (
							<span className="truncate text-xs text-muted-foreground">
								{artifact.created_by}
							</span>
						)}
						<Sev tone="info">{kindLabel(artifact.kind)}</Sev>
					</div>
					{artifact.card_title ? (
						<span className="truncate text-xs text-muted-foreground">
							{artifact.card_title}
						</span>
					) : null}
					<span className="text-[11.5px] text-faint">
						Mis à jour {relativeAge(artifact.updated_at)}
					</span>
				</div>
			</button>
			<Link
				to="/artifacts/$artifactId"
				params={{ artifactId: artifact.id }}
				aria-label={`Ouvrir en pleine page : ${artifact.title}`}
				title="Ouvrir en pleine page"
				className="absolute top-2 right-2 grid size-7 place-items-center rounded-md border bg-card text-muted-foreground transition-colors duration-150 hover:bg-accent hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
			>
				<Maximize2 aria-hidden="true" className="size-3.5" />
			</Link>
		</article>
	)
}
