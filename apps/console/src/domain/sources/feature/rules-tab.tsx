import { useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import {
	type DemoSource,
	shortName,
	updateSource,
	useDemoState,
} from '#/domain/demo'
import { testValue } from '#/domain/sources/glob'
import { typeInfo } from '#/domain/sources/source-form'
import { AddRow, CTL } from '#/domain/sources/ui/mockup'
import { PatternList } from '#/domain/sources/ui/pattern-list'
import { cn } from '#/lib/utils'

const OK_BOX =
	'rounded-[7px] border border-signal-success/45 bg-signal-success/10 px-2.5 py-2 text-signal-success'

interface Props {
	source: DemoSource
}

export function RulesTab({ source }: Props) {
	const agents = useDemoState((s) => s.agents)
	const [value, setValue] = useState('')
	const verdict = testValue(source, value)
	const first = agents.find((a) => a.id === source.agentIds[0])
	const placeholder = typeInfo(source.type).placeholder.split(',')[0] ?? ''
	return (
		<div>
			<SectionCard
				title="Motifs à écouter"
				description="* = n'importe quels caractères"
				flush
			>
				<PatternList
					items={source.patterns}
					placeholder={placeholder}
					addLabel="Nouveau motif"
					onRemove={(item) =>
						updateSource(source.id, {
							patterns: source.patterns.filter((p) => p !== item),
						})
					}
					onAdd={(item) =>
						updateSource(source.id, { patterns: [...source.patterns, item] })
					}
				/>
			</SectionCard>
			<SectionCard
				title="Exclusions"
				description="Prioritaires sur les motifs"
				flush
			>
				<PatternList
					items={source.exclusions}
					placeholder="*.getnatalia.com"
					addLabel="Nouvelle exclusion"
					onRemove={(item) =>
						updateSource(source.id, {
							exclusions: source.exclusions.filter((p) => p !== item),
						})
					}
					onAdd={(item) =>
						updateSource(source.id, {
							exclusions: [...source.exclusions, item],
						})
					}
				/>
			</SectionCard>
			<SectionCard title="Tester une valeur" flush>
				<AddRow className="pt-3">
					<input
						className={cn(CTL, 'flex-1 font-mono text-[11.5px]')}
						aria-label="Valeur à tester"
						placeholder={
							source.type === 'DNS' ? 'auth.exemple.fr' : 'valeur à tester'
						}
						value={value}
						onChange={(e) => setValue(e.target.value)}
					/>
				</AddRow>
				<div aria-live="polite" className="px-3 pb-3">
					{verdict === null ? (
						<span className="text-muted-foreground">
							Saisis un nom d'hôte pour voir ce que les règles en feraient.
						</span>
					) : verdict.kind === 'match' ? (
						<div className={OK_BOX}>
							Correspond à{' '}
							<b className="font-mono text-[11.5px]">{verdict.pattern}</b>.{' '}
							{first
								? `${shortName(first.name)} recevrait une tâche.`
								: "Aucun agent n'écoute : rien ne serait créé."}
						</div>
					) : verdict.kind === 'excluded' ? (
						<div className={cn(OK_BOX, 'border-border bg-secondary')}>
							Ignoré : exclu par{' '}
							<b className="font-mono text-[11.5px]">{verdict.by}</b>.
						</div>
					) : (
						<div className={cn(OK_BOX, 'border-border bg-secondary')}>
							Aucun motif ne correspond. L'événement est journalisé, sans tâche.
						</div>
					)}
				</div>
			</SectionCard>
		</div>
	)
}
