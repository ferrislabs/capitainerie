import { useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import type { DemoSource } from '#/domain/demo'

interface Props {
	listened: ReadonlyArray<DemoSource>
	available: ReadonlyArray<DemoSource>
	onDetach: (sourceId: string) => void
	onAttach: (sourceId: string) => void
}

export function SourcesTab({ listened, available, onDetach, onAttach }: Props) {
	const [chosen, setChosen] = useState<string>()
	const selected = available.some((s) => s.id === chosen)
		? chosen
		: available[0]?.id
	const first = listened[0]
	return (
		<div className="flex flex-col gap-4">
			<SectionCard title="Sources écoutées" flush>
				{listened.length === 0 ? (
					<p className="px-3 py-4 text-muted-foreground">
						Cet agent n'écoute aucune source. Il ne se réveille que sur planning
						ou sur assignation manuelle.
					</p>
				) : (
					<ul className="divide-y">
						{listened.map((source) => (
							<li
								key={source.id}
								className="flex items-center gap-3 px-3 py-2.5"
							>
								<span className="min-w-0 flex-1">
									<b className="block truncate font-medium">{source.name}</b>
									<span className="block truncate font-mono text-xs text-muted-foreground">
										{source.patterns.join('  ') || '—'}
									</span>
								</span>
								<span className="text-xs text-muted-foreground">
									{source.eventsLast24h} / 24 h
								</span>
								<Badge variant={source.enabled ? 'success' : 'neutral'}>
									{source.enabled ? 'Active' : 'En pause'}
								</Badge>
								<Button
									type="button"
									variant="outline"
									size="sm"
									aria-label={`Arrêter d'écouter ${source.name}`}
									onClick={() => onDetach(source.id)}
								>
									Arrêter d'écouter
								</Button>
							</li>
						))}
					</ul>
				)}
			</SectionCard>
			<SectionCard title="Écouter une autre source">
				{available.length === 0 ? (
					<p className="text-muted-foreground">
						Toutes les sources existantes sont déjà écoutées.
					</p>
				) : (
					<div className="flex gap-2">
						<NativeSelect
							aria-label="Source"
							value={selected}
							onChange={(event) => setChosen(event.target.value)}
						>
							{available.map((source) => (
								<option key={source.id} value={source.id}>
									{source.name} ({source.type})
								</option>
							))}
						</NativeSelect>
						<Button
							type="button"
							disabled={!selected}
							onClick={() => selected && onAttach(selected)}
						>
							Écouter
						</Button>
					</div>
				)}
			</SectionCard>
			{first ? (
				<SectionCard title="À chaque événement" flush>
					<ul className="divide-y">
						<li className="px-3 py-2.5">
							<b className="block font-medium">
								À chaque événement : une tâche est créée et assignée à cet agent
							</b>
							<span className="font-mono text-xs text-muted-foreground">
								{first.template}
							</span>
						</li>
						<li className="px-3 py-2.5">
							<b className="block font-medium">Réveil immédiat</b>
							<span className="text-xs text-muted-foreground">
								L'agent n'attend pas son prochain heartbeat. Les doublons sont
								ignorés pendant {first.dedupe}.
							</span>
						</li>
					</ul>
				</SectionCard>
			) : null}
		</div>
	)
}
