import { Link } from '@tanstack/react-router'
import { Radio } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { EmptyState } from '#/components/layout/empty-state'
import { Notice, useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { TabNav } from '#/components/layout/tab-nav'
import { Button } from '#/components/ui/button'
import { updateSource, useDemoState } from '#/domain/demo'
import { AgentsTab } from '#/domain/sources/feature/agents-tab'
import { EventsTab } from '#/domain/sources/feature/events-tab'
import { OverviewTab } from '#/domain/sources/feature/overview-tab'
import { PipelineTab } from '#/domain/sources/feature/pipeline-tab'
import { RulesTab } from '#/domain/sources/feature/rules-tab'
import { typeInfo } from '#/domain/sources/source-form'
import { resolveTab, SOURCE_TABS } from '#/domain/sources/tabs'
import { SourceTypeTile } from '#/domain/sources/ui/source-type-tile'

interface Props {
	sourceId: string
	tab?: string
}

export function SourceDetailPage({ sourceId, tab }: Props) {
	const source = useDemoState((s) => s.sources.find((x) => x.id === sourceId))
	const [message, notify] = useNotice()
	const current = resolveTab(tab)
	if (!source) {
		return (
			<Page>
				<DemoBanner />
				<EmptyState
					icon={Radio}
					title="Source introuvable"
					description="Cette source n'existe pas ou a été supprimée."
					action={
						<Button asChild variant="outline">
							<Link to="/sources">Retour aux sources</Link>
						</Button>
					}
				/>
			</Page>
		)
	}
	const tabs = SOURCE_TABS.map((t) => ({
		label: t.label,
		link: {
			to: '/sources/$sourceId/{-$tab}' as const,
			params:
				t.id === 'apercu' && current === 'apercu'
					? { sourceId, tab }
					: { sourceId, tab: t.id },
		},
	}))
	return (
		<Page>
			<DemoBanner />
			<header className="mb-3 flex flex-wrap items-end gap-x-3.5 gap-y-2.5">
				<SourceTypeTile type={source.type} size={36} />
				<div className="min-w-0">
					<h1 className="m-0 text-xl leading-[1.2] font-semibold tracking-[-0.01em]">
						{source.name}
					</h1>
					<span className="text-muted-foreground">
						{`${typeInfo(source.type).label} · ${source.enabled ? 'Active' : 'En pause'} · dernier événement ${source.last}`}
					</span>
				</div>
				<div className="ml-auto flex flex-wrap items-center gap-2">
					<Button
						type="button"
						variant="outline"
						onClick={() => {
							updateSource(source.id, { enabled: !source.enabled })
							notify(
								`${source.enabled ? 'Source en pause' : 'Source activée'} : ${source.name}`,
							)
						}}
					>
						{source.enabled ? 'Mettre en pause' : 'Activer'}
					</Button>
					<Button
						type="button"
						variant="outline"
						onClick={() =>
							notify(`Événement de test envoyé : ${source.patterns[0] ?? ''}`)
						}
					>
						Envoyer un test
					</Button>
				</div>
			</header>
			<TabNav label="Sections de la source" tabs={tabs} />
			{current === 'apercu' ? <OverviewTab source={source} /> : null}
			{current === 'regles' ? <RulesTab source={source} /> : null}
			{current === 'pipeline' ? (
				<PipelineTab source={source} notify={notify} />
			) : null}
			{current === 'evenements' ? <EventsTab sourceId={source.id} /> : null}
			{current === 'agents' ? (
				<AgentsTab source={source} notify={notify} />
			) : null}
			<Notice message={message} />
		</Page>
	)
}
