import { Link } from '@tanstack/react-router'
import { UserX } from 'lucide-react'
import { type ReactNode, useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { EmptyState } from '#/components/layout/empty-state'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Notice, useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { Button } from '#/components/ui/button'
import { Skeleton } from '#/components/ui/skeleton'
import {
	type AgentForm,
	buildForm,
	diffForm,
} from '#/domain/agent-team/agent-config'
import {
	type AgentTabId,
	resolveAgentTab,
} from '#/domain/agent-team/agent-tabs'
import { DIRECTION } from '#/domain/agent-team/constants'
import {
	useActiveRuns,
	useAgentCards,
	useRestoreTeamAgentRevision,
	useTeamAgent,
	useTeamAgentRevisions,
	useTeamAgents,
	useUpdateTeamAgent,
} from '#/domain/agent-team/hooks/use-team-agents'
import { isValidModelId, MODEL_ID_ERROR } from '#/domain/agent-team/model-id'
import { ancestorsOf, subAgents } from '#/domain/agent-team/org-tree'
import { agentPatch } from '#/domain/agent-team/save-agent'
import {
	describeAgentError,
	type TeamAgent,
} from '#/domain/agent-team/team-agent'
import { AgentHeader } from '#/domain/agent-team/ui/agent-header'
import { AgentTabBar } from '#/domain/agent-team/ui/agent-tab-bar'
import { ConnectorsTab } from '#/domain/agent-team/ui/connectors-tab'
import {
	type DelegatedTask,
	DelegationTab,
} from '#/domain/agent-team/ui/delegation-tab'
import {
	type InstructionsMode,
	InstructionsTab,
} from '#/domain/agent-team/ui/instructions-tab'
import { OverviewTab } from '#/domain/agent-team/ui/overview-tab'
import { PermsTab } from '#/domain/agent-team/ui/perms-tab'
import { RevisionsTab } from '#/domain/agent-team/ui/revisions-tab'
import { RuntimeTab } from '#/domain/agent-team/ui/runtime-tab'
import { SaveBar } from '#/domain/agent-team/ui/save-bar'
import { SkillsTab } from '#/domain/agent-team/ui/skills-tab'
import { SourcesTab } from '#/domain/agent-team/ui/sources-tab'
import { useMcpConnectors } from '#/domain/connectors/hooks/use-mcp-connectors'
import { useSkillCatalog } from '#/domain/skills/hooks/use-skills'
import { NewTaskDialog } from '#/domain/tasks/feature/new-task-dialog'
import { useCards } from '#/domain/tasks/hooks/use-tasks'
import { type Task, toTask } from '#/domain/tasks/model'
import { shortName } from '#/lib/agent-name'

interface Props {
	agentId: string
	tab?: string
	mode?: InstructionsMode
}

interface BodyProps {
	agent: TeamAgent
	agents: ReadonlyArray<TeamAgent>
	tab: AgentTabId
	mode: InstructionsMode
}

const CLOSED = new Set(['done', 'cancelled'])
const SAVE_TABS: ReadonlyArray<AgentTabId> = [
	'instructions',
	'team',
	'connectors',
	'runtime',
	'perms',
]
const RECENT = 6

const byRecent = (a: Task, b: Task) =>
	Date.parse(b.card.updated_at) - Date.parse(a.card.updated_at)

function AgentDetailBody({ agent, agents, tab, mode }: BodyProps) {
	const update = useUpdateTeamAgent()
	const restore = useRestoreTeamAgentRevision()
	const revisions = useTeamAgentRevisions(agent.id, tab === 'rev')
	const agentCards = useAgentCards(agent.id)
	const allCards = useCards()
	const activeRuns = useActiveRuns()
	const [draft, setDraft] = useState<AgentForm | null>(null)
	const [assignTo, setAssignTo] = useState<string | null>(null)
	const [message, setMessage] = useNotice()
	const connectors = useMcpConnectors(false)
	const skillCatalog = useSkillCatalog(tab === 'skills')

	const base = buildForm(agent)
	const form = draft ?? base
	const dirty = diffForm(base, form).length > 0
	const patch = (change: Partial<AgentForm>) => setDraft({ ...form, ...change })
	const short = shortName(agent.name)

	const pool = agents.some((a) => a.id === agent.id)
		? agents
		: [agent, ...agents]
	const subs = subAgents(pool, agent.id)
	const ancestors = ancestorsOf(pool, agent.id)
	const parent = pool.find((a) => a.id === agent.reports_to)
	const granted = agent.connector_grants

	const now = Date.now()
	const ownTasks = (agentCards.data?.items ?? [])
		.map((card) => toTask(card, now))
		.sort(byRecent)
	const ownIds = new Set(ownTasks.map((t) => t.id))
	const allTasks = (allCards.data?.items ?? []).map((card) => toTask(card, now))
	const runs = (activeRuns.data?.items ?? []).filter(
		(r) => r.run.card_id && ownIds.has(r.run.card_id),
	)

	const send = (
		body: Schemas.UpdateTeamAgentRequest,
		target: string,
		success: string,
	) =>
		update.mutate(
			{ path: { agent_id: target }, body },
			{
				onSuccess: () => setMessage(success),
				onError: (error) => setMessage(describeAgentError(error)),
			},
		)

	const save = () => {
		if (form.model !== base.model && !isValidModelId(form.model)) {
			setMessage(MODEL_ID_ERROR)
			return
		}
		const body = agentPatch(agent, base, form)
		if (Object.keys(body).length === 0) {
			setDraft(null)
			return
		}
		update.mutate(
			{ path: { agent_id: agent.id }, body },
			{
				onSuccess: (saved) => {
					setDraft(null)
					setMessage(`Enregistré · révision ${saved.revision} créée`)
				},
				onError: (error) => setMessage(describeAgentError(error)),
			},
		)
	}

	const toggleConnector = (slug: string, allowed: boolean) => {
		if (!allowed) {
			send(
				{
					connector_grants: granted.filter((c) => c !== slug),
					change_summary: 'connecteurs',
				},
				agent.id,
				`Accès retiré à ${short}`,
			)
			return
		}
		if (granted.includes(slug)) return
		const label =
			connectors.data?.items.find((c) => c.slug === slug)?.name ?? slug
		send(
			{ connector_grants: [...granted, slug], change_summary: 'connecteurs' },
			agent.id,
			`${short} peut utiliser ${label}`,
		)
	}

	const setSkills = (skills: ReadonlyArray<string>, success: string) =>
		send(
			{ skills: [...skills], change_summary: 'compétences' },
			agent.id,
			success,
		)

	const toggleArchive = () =>
		send(
			{
				archived: !agent.archived,
				change_summary: agent.archived ? 'restauration' : 'archivage',
			},
			agent.id,
			agent.archived ? `${short} restauré` : `${short} archivé`,
		)

	const togglePause = () =>
		send(
			{
				paused: !agent.paused,
				change_summary: agent.paused ? 'reprise' : 'pause',
			},
			agent.id,
			agent.paused ? 'Agent relancé' : 'Agent mis en pause',
		)

	const removeSub = (subId: string) =>
		send(
			{ reports_to: parent?.id ?? null, change_summary: 'hiérarchie' },
			subId,
			'Sous-agent rattaché au parent de cet agent',
		)

	const restoreRevision = (revision: number) =>
		restore.mutate(
			{ path: { agent_id: agent.id, revision } },
			{
				onSuccess: (restored) => {
					setDraft(null)
					setMessage(
						`Révision ${revision} restaurée (révision ${restored.revision} créée)`,
					)
				},
				onError: (error) => setMessage(describeAgentError(error)),
			},
		)

	const taskById = new Map(allTasks.map((t) => [t.id, t]))
	const recent: DelegatedTask[] = allTasks
		.filter((t) => t.parentId && taskById.get(t.parentId)?.agentId === agent.id)
		.sort(byRecent)
		.slice(0, RECENT)
		.map((task) => ({
			task,
			assignee: pool.find((a) => a.id === task.agentId),
		}))

	const others = pool
		.filter((a) => a.id !== agent.id)
		.map((a) => ({ value: a.id, label: a.name }))

	const openTasks = (id: string) =>
		allTasks.filter((t) => t.agentId === id && !CLOSED.has(t.status)).length

	const showBar =
		dirty &&
		SAVE_TABS.includes(tab) &&
		(tab !== 'instructions' || mode === 'editer')

	return (
		<Page>
			<AgentHeader
				agent={agent}
				adapter={form.adapter}
				title={form.title}
				paused={agent.paused}
				busy={update.isPending}
				onAssign={() => setAssignTo(agent.id)}
				onTogglePause={togglePause}
				onToggleArchive={toggleArchive}
			/>
			<AgentTabBar agentId={agent.id} active={tab} />
			<Notice message={message} />
			{tab === 'overview' ? (
				<OverviewTab
					agent={agent}
					form={form}
					subCount={subs.length}
					reportsTo={parent?.name ?? DIRECTION}
					tasks={ownTasks.slice(0, RECENT)}
					tasksLoading={agentCards.isPending}
					tasksError={agentCards.isError}
					activeRuns={runs}
				/>
			) : null}
			{tab === 'instructions' ? (
				<InstructionsTab
					agentId={agent.id}
					mode={mode}
					instr={form.instr}
					onChange={(instr) => patch({ instr })}
				/>
			) : null}
			{tab === 'skills' ? (
				<SkillsTab
					skills={agent.skills}
					catalog={skillCatalog}
					busy={update.isPending}
					onAdd={(skill) =>
						setSkills([...agent.skills, skill], `${skill} activée`)
					}
					onRemove={(skill) =>
						setSkills(
							agent.skills.filter((s) => s !== skill),
							`${skill} désactivée`,
						)
					}
				/>
			) : null}
			{tab === 'sources' ? <SourcesTab agentId={agent.id} /> : null}
			{tab === 'team' ? (
				<DelegationTab
					agent={agent}
					ancestors={ancestors}
					subs={subs.map((sub) => ({
						agent: sub,
						openTasks: openTasks(sub.id),
					}))}
					candidates={pool.filter(
						(a) =>
							a.id !== agent.id &&
							!subs.some((sub) => sub.id === a.id) &&
							!ancestors.some((anc) => anc.id === a.id),
					)}
					recent={recent}
					form={form}
					onChange={patch}
					onRemove={removeSub}
					onAssign={(sub) => setAssignTo(sub.id)}
					onAdd={(id) =>
						send(
							{ reports_to: agent.id, change_summary: 'hiérarchie' },
							id,
							'Sous-agent ajouté',
						)
					}
				/>
			) : null}
			{tab === 'connectors' ? (
				<ConnectorsTab
					connectors={connectors.data?.items ?? []}
					loading={connectors.isPending}
					error={connectors.isError}
					granted={granted}
					form={form}
					onToggle={toggleConnector}
					onChange={patch}
				/>
			) : null}
			{tab === 'runtime' ? (
				<RuntimeTab form={form} reportsOptions={others} onChange={patch} />
			) : null}
			{tab === 'perms' ? <PermsTab form={form} onChange={patch} /> : null}
			{tab === 'rev' ? (
				<RevisionsTab
					current={agent.revision}
					revisions={revisions.data?.items ?? []}
					loading={revisions.isPending}
					error={revisions.isError}
					restoring={restore.isPending}
					onRestore={restoreRevision}
				/>
			) : null}

			{showBar ? (
				<SaveBar
					busy={update.isPending}
					onCancel={() => setDraft(null)}
					onSave={save}
				/>
			) : null}
			{assignTo !== null ? (
				<NewTaskDialog
					key={assignTo}
					open
					defaultAgentId={assignTo}
					onOpenChange={(open) => {
						if (!open) setAssignTo(null)
					}}
				/>
			) : null}
		</Page>
	)
}

function Placeholder({ children }: { children: ReactNode }) {
	return <Page>{children}</Page>
}

export function AgentDetailPage({ agentId, tab, mode = 'lire' }: Props) {
	const query = useTeamAgent(agentId)
	const team = useTeamAgents(false)
	if (query.isPending) {
		return (
			<Placeholder>
				<output className="grid gap-2.5">
					<span className="sr-only">Chargement…</span>
					<Skeleton className="h-10 w-72" />
					<Skeleton className="h-48 rounded-lg" />
				</output>
			</Placeholder>
		)
	}
	if (query.isError || !query.data) {
		const notFound = (query.error as { status?: number } | null)?.status === 404
		if (notFound) {
			return (
				<Placeholder>
					<EmptyState
						icon={UserX}
						title="Agent introuvable"
						description="Cet agent n'existe pas ou n'appartient pas à cette organisation."
						action={
							<Button asChild variant="outline">
								<Link to="/agents">Retour à l'équipe</Link>
							</Button>
						}
					/>
				</Placeholder>
			)
		}
		return (
			<Placeholder>
				<InlineAlert tone="danger">Impossible de charger l'agent.</InlineAlert>
			</Placeholder>
		)
	}
	return (
		<AgentDetailBody
			key={query.data.id}
			agent={query.data}
			agents={team.data?.items ?? []}
			tab={resolveAgentTab(tab)}
			mode={mode}
		/>
	)
}
