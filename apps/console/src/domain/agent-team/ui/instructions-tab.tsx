import { Link } from '@tanstack/react-router'
import { FileText } from 'lucide-react'
import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import { Textarea } from '#/components/ui/textarea'
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

function Rendered({ text }: { text: string }) {
	return (
		<div className="flex flex-col gap-1.5 p-3">
			{text.split('\n').map((line, index) => {
				const key = `${index}-${line}`
				if (!line.trim()) return null
				if (line.startsWith('## '))
					return (
						<h3 key={key} className="mt-2 text-[13px] font-semibold">
							{line.slice(3)}
						</h3>
					)
				if (line.startsWith('# '))
					return (
						<h2 key={key} className="text-base font-semibold">
							{line.slice(2)}
						</h2>
					)
				return (
					<p
						key={key}
						className={cn(
							/^(- |\d+\. )/.test(line) && 'pl-3 text-muted-foreground',
						)}
					>
						{line}
					</p>
				)
			})}
		</div>
	)
}

export function InstructionsTab({ agentId, mode, instr, onChange }: Props) {
	return (
		<div className="grid gap-4 lg:grid-cols-[12.5rem_minmax(0,1fr)]">
			<SectionCard title="Fichiers" flush>
				<div className="flex items-center gap-2 px-3 py-2">
					<FileText aria-hidden="true" className="size-4" />
					AGENTS.md
					<Badge variant="muted" className="ml-auto">
						entrée
					</Badge>
				</div>
			</SectionCard>
			<SectionCard
				title="AGENTS.md"
				flush
				actions={
					<nav aria-label="Mode d'affichage" className="flex gap-1">
						{MODES.map((m) => (
							<Link
								key={m.id}
								to="/agents/$agentId/{-$tab}"
								params={{ agentId, tab: 'instructions' }}
								search={{ mode: m.id }}
								aria-current={m.id === mode ? 'page' : undefined}
								className={cn(
									'rounded-md px-2.5 py-0.5 text-xs font-medium text-muted-foreground hover:text-foreground',
									m.id === mode && 'bg-accent text-foreground',
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
						<Textarea
							aria-label="Instructions"
							className="font-mono"
							rows={16}
							value={instr}
							onChange={(event) => onChange(event.target.value)}
						/>
					</div>
				) : mode === 'brut' ? (
					<pre className="overflow-auto p-3 font-mono text-xs whitespace-pre-wrap">
						{instr}
					</pre>
				) : (
					<Rendered text={instr} />
				)}
			</SectionCard>
		</div>
	)
}
