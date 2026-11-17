import { Link } from '@tanstack/react-router'
import { FileText, Plus } from 'lucide-react'
import { SectionCard } from '#/components/layout/section-card'
import { CTL } from '#/domain/agent-team/ui/field-row'
import { cn } from '#/lib/utils'

export type InstructionsMode = 'lire' | 'editer' | 'brut'

const MODES: ReadonlyArray<{ id: InstructionsMode; label: string }> = [
	{ id: 'lire', label: 'Lire' },
	{ id: 'editer', label: 'Éditer' },
	{ id: 'brut', label: 'Brut' },
]

interface Props {
	agentId: string
	mode: InstructionsMode
	instr: string
	onChange: (instr: string) => void
	onNotify: (message: string) => void
}

const DOC =
	'm-0 whitespace-pre-wrap px-3.5 py-3 font-[inherit] leading-[1.4] [overflow-wrap:anywhere]'

export function InstructionsTab({
	agentId,
	mode,
	instr,
	onChange,
	onNotify,
}: Props) {
	return (
		<div className="grid grid-cols-[200px_minmax(0,1fr)] items-start gap-2.5">
			<SectionCard
				title="Fichiers"
				flush
				actions={
					<button
						type="button"
						aria-label="Ajouter"
						onClick={() => onNotify("Nouveau fichier d'instructions")}
						className="grid size-[22px] place-items-center rounded-md text-muted-foreground hover:bg-secondary hover:text-foreground"
					>
						<Plus aria-hidden="true" className="size-4" />
					</button>
				}
			>
				<div className="grid content-start gap-0.5 p-2">
					<div className="flex h-7 items-center gap-[7px] rounded-md bg-secondary px-2 text-left">
						<FileText aria-hidden="true" className="size-4" />
						AGENTS.md
						<span className="ml-auto rounded-lg bg-secondary px-1.5 text-[11px] leading-4 tabular-nums">
							entrée
						</span>
					</div>
				</div>
			</SectionCard>
			<SectionCard
				title="AGENTS.md"
				flush
				actions={
					<nav
						aria-label="Mode d'affichage"
						className="inline-flex overflow-hidden rounded-md border"
					>
						{MODES.map((m) => (
							<Link
								key={m.id}
								to="/agents/$agentId/{-$tab}"
								params={{ agentId, tab: 'instructions' }}
								search={{ mode: m.id }}
								aria-current={m.id === mode ? 'page' : undefined}
								className={cn(
									'grid h-7 place-items-center bg-card px-2.5 text-xs text-primary',
									m.id === mode && 'bg-secondary text-foreground',
								)}
							>
								{m.label}
							</Link>
						))}
					</nav>
				}
			>
				{mode === 'editer' ? (
					<div className="p-2.5">
						<textarea
							aria-label="Instructions"
							className={cn(CTL, 'font-mono text-[11.5px]')}
							rows={16}
							value={instr}
							onChange={(event) => onChange(event.target.value)}
						/>
					</div>
				) : mode === 'brut' ? (
					<pre className={DOC}>{instr}</pre>
				) : (
					<div className={DOC}>{instr}</div>
				)}
			</SectionCard>
		</div>
	)
}
