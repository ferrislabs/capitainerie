import { Link, useNavigate } from '@tanstack/react-router'
import { Radio } from 'lucide-react'
import { useState } from 'react'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Notice, useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { SectionCard } from '#/components/layout/section-card'
import { TabNav } from '#/components/layout/tab-nav'
import { Button } from '#/components/ui/button'
import { relativeAge } from '#/domain/dashboard/format'
import { EventsPanel } from '#/domain/sources/feature/events-panel'
import { useSourceFormOptions } from '#/domain/sources/feature/use-form-options'
import {
	useArchiveSource,
	useRotateSourceToken,
	useSource,
	useTestSource,
	useUpdateSource,
} from '#/domain/sources/hooks/use-sources'
import {
	describeSourceError,
	draftOf,
	type IssuedSource,
	type Source,
	type SourceDraft,
	updateBody,
} from '#/domain/sources/model'
import { resolveTab, SOURCE_TABS } from '#/domain/sources/tabs'
import { Sev } from '#/domain/sources/ui/kit'
import { OverviewPanel } from '#/domain/sources/ui/overview-panel'
import { RotateTokenDialog } from '#/domain/sources/ui/rotate-token-dialog'
import { SourceFormView } from '#/domain/sources/ui/source-form-view'

interface Props {
	sourceId: string
	tab?: string
}

export function SourceDetailPage({ sourceId, tab }: Props) {
	const query = useSource(sourceId)
	if (query.isPending) {
		return (
			<Page>
				<output className="text-muted-foreground">
					Chargement de la source…
				</output>
			</Page>
		)
	}
	if (query.isError) {
		const missing = (query.error as { status?: number } | null)?.status === 404
		return (
			<Page>
				{missing ? (
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
				) : (
					<div className="grid gap-2">
						<InlineAlert tone="danger">
							Impossible de charger la source.
						</InlineAlert>
						<div>
							<Button variant="outline" onClick={() => void query.refetch()}>
								Réessayer
							</Button>
						</div>
					</div>
				)}
			</Page>
		)
	}
	return <SourceDetail key={query.data.id} source={query.data} tab={tab} />
}

function SourceDetail({ source, tab }: { source: Source; tab?: string }) {
	const navigate = useNavigate()
	const [message, notify] = useNotice()
	const [issued, setIssued] = useState<IssuedSource | null>(null)
	const [confirmRotate, setConfirmRotate] = useState(false)
	const [formError, setFormError] = useState<string | null>(null)
	const { agents, projects } = useSourceFormOptions()
	const update = useUpdateSource()
	const archive = useArchiveSource()
	const rotate = useRotateSourceToken()
	const test = useTestSource()
	const current = resolveTab(tab)
	const path = { source_id: source.id }
	const fail = (error: unknown) => notify(describeSourceError(error))

	const tabs = SOURCE_TABS.map((t) => ({
		label: t.label,
		link: {
			to: '/sources/$sourceId/{-$tab}' as const,
			params:
				t.id === 'apercu' && current === 'apercu'
					? { sourceId: source.id, tab }
					: { sourceId: source.id, tab: t.id },
		},
	}))

	const setEnabled = (enabled: boolean) =>
		update.mutate(
			{ path, body: { enabled } },
			{
				onSuccess: () =>
					notify(
						`${enabled ? 'Source activée' : 'Source en pause'} : ${source.name}`,
					),
				onError: fail,
			},
		)

	const sendTest = () =>
		test.mutate(
			{ path },
			{
				onSuccess: () => {
					notify('Événement de test envoyé')
					if (current !== 'evenements')
						void navigate({
							to: '/sources/$sourceId/{-$tab}',
							params: { sourceId: source.id, tab: 'evenements' },
						})
				},
				onError: fail,
			},
		)

	const doRotate = () =>
		rotate.mutate(
			{ path },
			{
				onSuccess: (next) => {
					setIssued(next)
					notify('Nouveau token généré : l’ancienne URL ne fonctionne plus')
				},
				onError: fail,
			},
		)

	const save = (draft: SourceDraft) => {
		setFormError(null)
		const body = updateBody(source, draft)
		if (Object.keys(body).length === 0) {
			notify('Aucune modification')
			return
		}
		update.mutate(
			{ path, body },
			{
				onSuccess: () => notify('Source enregistrée'),
				onError: (error) => setFormError(describeSourceError(error)),
			},
		)
	}

	const setArchived = (archived: boolean) => {
		const done = {
			onSuccess: () =>
				notify(archived ? 'Source archivée' : 'Source restaurée'),
			onError: fail,
		}
		if (archived) archive.mutate({ path }, done)
		else update.mutate({ path, body: { archived: false } }, done)
	}

	const agentName = agents.find((a) => a.id === source.default_agent_id)?.name
	const projectName = projects.find((p) => p.id === source.project_id)?.name
	const state = source.archived
		? 'Archivée'
		: source.enabled
			? 'Active'
			: 'En pause'
	const last = source.last_event_at
		? `dernier événement ${relativeAge(source.last_event_at)}`
		: 'aucun événement reçu'

	return (
		<Page>
			<header className="mb-3 flex flex-wrap items-end gap-x-3.5 gap-y-2.5">
				<div className="min-w-0">
					<h1 className="m-0 text-xl leading-[1.2] font-semibold tracking-[-0.01em]">
						{source.name}
					</h1>
					<span className="flex flex-wrap items-center gap-1.5 text-muted-foreground">
						<Sev
							tone={source.archived ? 'info' : source.enabled ? 'ok' : 'warn'}
						>
							{state}
						</Sev>
						{`Webhook · ${last}`}
					</span>
				</div>
				{source.archived ? null : (
					<div className="ml-auto flex flex-wrap items-center gap-2">
						<Button
							type="button"
							variant="outline"
							disabled={update.isPending}
							onClick={() => setEnabled(!source.enabled)}
						>
							{source.enabled ? 'Mettre en pause' : 'Activer'}
						</Button>
						<Button type="button" disabled={test.isPending} onClick={sendTest}>
							Envoyer un événement de test
						</Button>
					</div>
				)}
			</header>
			<Notice message={message} className="mb-3" />
			<TabNav label="Sections de la source" tabs={tabs} />
			{current === 'apercu' ? (
				<OverviewPanel
					source={source}
					agentName={agentName}
					projectName={projectName}
					issued={issued}
					rotating={rotate.isPending}
					onRotate={() => setConfirmRotate(true)}
					onDismissIssued={() => setIssued(null)}
				/>
			) : null}
			{current === 'evenements' ? <EventsPanel sourceId={source.id} /> : null}
			{current === 'parametres' ? (
				<div className="grid gap-4">
					{source.archived ? null : (
						<SourceFormView
							mode="edit"
							initial={draftOf(source)}
							agents={agents}
							projects={projects}
							pending={update.isPending}
							error={formError}
							onSubmit={save}
						/>
					)}
					<SectionCard
						title={source.archived ? 'Source archivée' : 'Archiver'}
						className="max-w-[880px]"
					>
						<div className="flex flex-wrap items-center gap-3">
							<p className="m-0 min-w-0 flex-1 text-muted-foreground">
								{source.archived
									? 'Restaurer la source réactive son webhook avec le même token.'
									: 'Le webhook ne répond plus ; les événements reçus restent consultables.'}
							</p>
							<Button
								type="button"
								variant="outline"
								disabled={archive.isPending || update.isPending}
								onClick={() => setArchived(!source.archived)}
							>
								{source.archived ? 'Restaurer' : 'Archiver la source'}
							</Button>
						</div>
					</SectionCard>
				</div>
			) : null}
			<RotateTokenDialog
				open={confirmRotate}
				name={source.name}
				onOpenChange={setConfirmRotate}
				onConfirm={() => {
					setConfirmRotate(false)
					doRotate()
				}}
			/>
		</Page>
	)
}
