import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { CTL } from '#/domain/agent-team/ui/field-row'
import { HumanAvatar } from '#/domain/agent-team/ui/human-avatar'
import { Empty, ROW, Rows, Ttl } from '#/domain/agent-team/ui/rows'
import { Sev } from '#/domain/agent-team/ui/sev'
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
		<>
			<SectionCard
				title="Sources écoutées"
				flush
				actions={<Link to="/sources/nouvelle">Nouvelle source</Link>}
			>
				{listened.length === 0 ? (
					<Empty title="Cet agent n'écoute aucune source">
						Il ne se réveille que sur planning ou sur assignation manuelle.
					</Empty>
				) : (
					<Rows>
						{listened.map((source) => (
							<div key={source.id} className={ROW}>
								<HumanAvatar initials={source.type.slice(0, 3)} size={30} />
								<Ttl
									title={
										<Link
											to="/sources/$sourceId/{-$tab}"
											params={{ sourceId: source.id }}
											className="text-foreground"
										>
											{source.name}
										</Link>
									}
									sub={source.patterns.join('  ') || '—'}
									subMono
								/>
								<span className="whitespace-nowrap text-muted-foreground tabular-nums">
									{source.eventsLast24h} / 24 h
								</span>
								<Sev tone={source.enabled ? 'ok' : 'info'}>
									{source.enabled ? 'Active' : 'En pause'}
								</Sev>
								<Button
									type="button"
									variant="outline"
									size="sm"
									aria-label={`Arrêter d'écouter ${source.name}`}
									onClick={() => onDetach(source.id)}
								>
									Arrêter d'écouter
								</Button>
							</div>
						))}
					</Rows>
				)}
			</SectionCard>
			<SectionCard title="Écouter une autre source" flush>
				{available.length === 0 ? (
					<p className="m-0 p-3 text-muted-foreground">
						Toutes les sources existantes sont déjà écoutées.
					</p>
				) : (
					<div className="flex gap-1.5 px-3 pt-3 pb-3">
						<select
							aria-label="Source"
							className={CTL}
							value={selected}
							onChange={(event) => setChosen(event.target.value)}
						>
							{available.map((source) => (
								<option key={source.id} value={source.id}>
									{source.name} ({source.type})
								</option>
							))}
						</select>
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
					<Rows>
						<div className={ROW}>
							<Ttl
								title="Une tâche est créée et assignée à cet agent"
								sub={first.template}
								subMono
							/>
						</div>
						<div className={ROW}>
							<Ttl
								title="Réveil immédiat"
								sub={`L'agent n'attend pas son prochain heartbeat. Les doublons sont ignorés pendant ${first.dedupe}.`}
							/>
						</div>
					</Rows>
				</SectionCard>
			) : null}
		</>
	)
}
