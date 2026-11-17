import { SectionCard } from '#/components/layout/section-card'
import { Tile } from '#/components/layout/tile'
import { type DemoSource, shortName, useDemoState } from '#/domain/demo'
import { eventsFor } from '#/domain/sources/mock/events'
import { typeInfo } from '#/domain/sources/source-form'
import { FlowStrip } from '#/domain/sources/ui/flow-strip'
import { KeyValueRows } from '#/domain/sources/ui/key-value-rows'

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
	const rows: Array<readonly [string, string]> = [
		['Type', typeInfo(source.type).label],
		['Données', source.feeds.join(', ')],
	]
	if (source.scope) rows.push(['Périmètre', source.scope])
	rows.push(
		['Création de tâches', source.mode],
		['Dédoublonnage', `Ignore une valeur déjà vue pendant ${source.dedupe}`],
		['Modèle de tâche', source.template],
	)
	return (
		<div className="flex flex-col gap-4">
			<div className="grid grid-cols-2 gap-2 lg:grid-cols-4">
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
			<SectionCard title="Chemin d'un événement">
				<FlowStrip
					steps={[
						{ title: '1. Source', body: source.feeds.join(' · ') },
						{ title: '2. Motifs', body: source.patterns.join('  ') || '—' },
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
