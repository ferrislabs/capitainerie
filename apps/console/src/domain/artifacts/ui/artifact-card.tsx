import { ChartLine, File } from 'lucide-react'
import type { Artifact } from '#/domain/artifacts/mock/artifacts'
import { DEMO_AGENTS, shortName } from '#/domain/demo'
import { AgentChip } from '#/domain/tasks/ui/agent-chip'
import { Sev } from '#/domain/tasks/ui/sev'

interface Props {
	artifact: Artifact
	onOpen: (artifact: Artifact) => void
}

export function ArtifactCard({ artifact, onOpen }: Props) {
	const Icon = artifact.type === 'Tableau de bord' ? ChartLine : File
	const hue = DEMO_AGENTS.find((a) => a.name === artifact.agent)?.hue ?? 250
	return (
		<article className="min-w-0 overflow-hidden rounded-lg border bg-card">
			<button
				type="button"
				onClick={() => onOpen(artifact)}
				className="block w-full cursor-pointer text-left"
			>
				<div className="grid h-[92px] place-items-center rounded-t-lg border-b bg-[repeating-linear-gradient(135deg,var(--secondary)_0_8px,var(--background)_8px_16px)] text-faint">
					<Icon aria-hidden="true" className="size-4" />
				</div>
				<div className="flex flex-col gap-[5px] px-3 py-[11px]">
					<h3 className="m-0 text-[13px] font-semibold">{artifact.name}</h3>
					<div className="flex justify-between gap-2">
						<AgentChip name={shortName(artifact.agent)} hue={hue} size={16} />
						<Sev tone="info">{artifact.type}</Sev>
					</div>
					<span className="text-[11.5px] text-faint">
						Mis à jour {artifact.updated}
					</span>
				</div>
			</button>
		</article>
	)
}
