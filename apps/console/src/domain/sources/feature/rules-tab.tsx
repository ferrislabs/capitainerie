import { useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { TextInput } from '#/components/ui/text-input'
import {
	type DemoSource,
	shortName,
	updateSource,
	useDemoState,
} from '#/domain/demo'
import { testValue } from '#/domain/sources/glob'
import { typeInfo } from '#/domain/sources/source-form'
import { PatternList } from '#/domain/sources/ui/pattern-list'

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
		<div className="flex flex-col gap-4">
			<SectionCard
				title="Motifs à écouter"
				description="* = n'importe quels caractères"
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
			<SectionCard title="Exclusions" description="Prioritaires sur les motifs">
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
			<SectionCard title="Tester une valeur">
				<div className="flex flex-col gap-3">
					<TextInput
						className="font-mono"
						aria-label="Valeur à tester"
						placeholder={
							source.type === 'DNS' ? 'auth.exemple.fr' : 'valeur à tester'
						}
						value={value}
						onChange={(e) => setValue(e.target.value)}
					/>
					<p aria-live="polite" className="text-muted-foreground">
						{verdict === null ? (
							"Saisis un nom d'hôte pour voir ce que les règles en feraient."
						) : verdict.kind === 'match' ? (
							<span className="text-foreground">
								Correspond à <b className="font-mono">{verdict.pattern}</b>.{' '}
								{first
									? `${shortName(first.name)} recevrait une tâche.`
									: "Aucun agent n'écoute : rien ne serait créé."}
							</span>
						) : verdict.kind === 'excluded' ? (
							<span className="text-foreground">
								Ignoré : exclu par <b className="font-mono">{verdict.by}</b>.
							</span>
						) : (
							"Aucun motif ne correspond. L'événement est journalisé, sans tâche."
						)}
					</p>
				</div>
			</SectionCard>
		</div>
	)
}
