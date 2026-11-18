import { Link } from '@tanstack/react-router'
import { FileText } from 'lucide-react'
import { CTL } from '#/domain/agent-team/ui/field-row'
import { DetailSection } from '#/domain/agent-team/ui/panel'
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
}

const DOC =
	'm-0 max-w-[80ch] whitespace-pre-wrap px-6 py-5 font-[inherit] text-[15px] leading-relaxed [overflow-wrap:anywhere]'

export function InstructionsTab({ agentId, mode, instr, onChange }: Props) {
	return (
		<div className="grid items-start gap-x-6 gap-y-10 min-[900px]:grid-cols-[220px_minmax(0,1fr)]">
			<DetailSection title="Fichiers" flush>
				<div className="grid content-start gap-0.5 p-3">
					<div className="flex h-9 items-center gap-2 rounded-md bg-secondary px-2.5 text-left">
						<FileText aria-hidden="true" className="size-4" />
						AGENTS.md
						<span className="ml-auto rounded-lg bg-secondary px-1.5 text-[11px] leading-4 tabular-nums">
							entrée
						</span>
					</div>
				</div>
			</DetailSection>
			<DetailSection
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
					<div className="p-5">
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
			</DetailSection>
		</div>
	)
}
