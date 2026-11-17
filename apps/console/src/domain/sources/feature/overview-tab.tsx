import type { ReactNode } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Tile } from '#/components/layout/tile'
import { type DemoSource, shortName, useDemoState } from '#/domain/demo'
import { eventsFor } from '#/domain/sources/mock/events'
import { typeInfo } from '#/domain/sources/source-form'
import { KeyValueRows } from '#/domain/sources/ui/key-value-rows'
import { Flow } from '#/domain/sources/ui/mockup'

interface Props {
	source: DemoSource
}

export function OverviewTab({ source }: Props) {
	const agents = useDemoState((s) => s.agents)
	const events = eventsFor(source.id)
	const count = (...kinds: string[]) =>
		events.filter((e) => kinds.includes(e.decision)).length
	const names = source.agentIds
		.map((id) => agents.find((a) => a.id === id))
		.filter((a) => a !== undefined)
		.map((a) => shortName(a.name))
	const rows: Array<readonly [string, ReactNode]> = [
		['Type', typeInfo(source.type).label],
		['Données', source.feeds.join(', ')],
	]
	if (source.scope) rows.push(['Périmètre', source.scope])
	rows.push(
		['Création de tâches', source.mode],
		['Dédoublonnage', `Ignore une valeur déjà vue pendant ${source.dedupe}`],
		[
			'Modèle de tâche',
			<span key="t" className="font-mono text-[11.5px]">
				{source.template}
			</span>,
		],
	)
	return (
		<div>
			<div className="mb-2.5 grid grid-cols-4 gap-2.5 max-[1100px]:grid-cols-2">
				<Tile
					label="Événements 24 h"
					value={source.eventsLast24h}
					hint="reçus"
				/>
				<Tile
					label="Tâches créées"
					value={count('done', 'progress', 'review')}
					hint="sur les derniers événements"
				/>
				<Tile label="Ignorés" value={count('ignored')} hint="exclusions" />
				<Tile
					label="Doublons"
					value={count('dup')}
					hint={`fenêtre ${source.dedupe}`}
				/>
			</div>
			<SectionCard title="Chemin d'un événement" flush>
				<Flow
					steps={[
						{ title: '1. Source', body: source.feeds.join(' · ') },
						{
							title: '2. Motifs',
							body: (
								<span className="font-mono text-[11.5px]">
									{source.patterns.join('  ') || '—'}
								</span>
							),
						},
						{
							title: '3. Agent',
							body: names.length > 0 ? names.join(', ') : 'Aucun agent',
						},
						{ title: '4. Tâche', body: source.template },
					]}
				/>
			</SectionCard>
			<SectionCard title="Configuration" flush>
				<KeyValueRows rows={rows} />
			</SectionCard>
		</div>
	)
}
