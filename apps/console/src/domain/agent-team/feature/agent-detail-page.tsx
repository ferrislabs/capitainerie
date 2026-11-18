import { Link } from '@tanstack/react-router'
import { UserX } from 'lucide-react'
import { type ReactNode, useState } from 'react'
import type { Schemas } from '#/api/api.client'
import { DemoBanner } from '#/components/layout/demo-banner'
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
	updateAgentConfig,
	useAgentConfig,
} from '#/domain/agent-team/agent-config-store'
import {
	type AgentTabId,
	resolveAgentTab,
} from '#/domain/agent-team/agent-tabs'
import { DIRECTION } from '#/domain/agent-team/constants'
import {
	useTeamAgent,
	useTeamAgents,
	useUpdateTeamAgent,
} from '#/domain/agent-team/hooks/use-team-agents'
import { isValidModelId, MODEL_ID_ERROR } from '#/domain/agent-team/model-id'
import { ancestorsOf, subAgents } from '#/domain/agent-team/org-tree'
import { agentPatch, recordRevision } from '#/domain/agent-team/save-agent'
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
import { toggled } from '#/domain/agent-team/wizard'
import { useMcpConnectors } from '#/domain/connectors/hooks/use-mcp-connectors'
import {
	attachSource,
	detachSource,
	shortName,
	useDemoState,
} from '#/domain/demo'

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

function AgentDetailBody({ agent, agents, tab, mode }: BodyProps) {
	const state = useDemoState((s) => s)
	const config = useAgentConfig(agent.id, agent)
	const update = useUpdateTeamAgent()
	const [draft, setDraft] = useState<AgentForm | null>(null)
	const [message, setMessage] = useNotice()
	const connectors = useMcpConnectors(false)
	const [testResult, setTestResult] = useState<string>()

	const base = buildForm(agent, config)
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

	const send = (
		body: Schemas.UpdateTeamAgentRequest,
		target: string,
		success: string,
		onDone?: () => void,
	) =>
		update.mutate(
			{ path: { agent_id: target }, body },
			{
				onSuccess: () => {
					onDone?.()
					setMessage(success)
				},
				onError: (error) => setMessage(describeAgentError(error)),
			},
		)

	const save = () => {
		if (form.model !== base.model && !isValidModelId(form.model)) {
			setMessage(MODEL_ID_ERROR)
			return
		}
		const body = agentPatch(base, form)
		const done = () => {
			const rev = recordRevision(agent, base, form)
			setDraft(null)
			setMessage(`Enregistré · révision ${rev} créée`)
		}
		if (Object.keys(body).length === 0) {
			done()
			return
		}
		update.mutate(
			{ path: { agent_id: agent.id }, body },
			{
				onSuccess: done,
				onError: (error) => setMessage(describeAgentError(error)),
			},
		)
	}

	const toggleConnector = (slug: string, allowed: boolean) => {
		if (!allowed) {
			send(
				{ connector_grants: granted.filter((c) => c !== slug) },
				agent.id,
				`Accès retiré à ${short}`,
			)
			return
		}
		if (granted.includes(slug)) return
		const label =
			connectors.data?.items.find((c) => c.slug === slug)?.name ?? slug
		send(
			{ connector_grants: [...granted, slug] },
			agent.id,
			`${short} peut utiliser ${label}`,
		)
	}

	const toggleArchive = () =>
		send(
			{ archived: !agent.archived },
			agent.id,
			agent.archived ? `${short} restauré` : `${short} archivé`,
		)

	const removeSub = (subId: string) =>
		send(
			{ reports_to: parent?.id ?? null },
			subId,
			'Sous-agent rattaché au parent de cet agent',
		)

	const taskOf = (id: string) => state.tasks.find((t) => t.id === id)
	const recent: DelegatedTask[] = state.tasks
		.filter((t) => t.parentId && taskOf(t.parentId)?.agentId === agent.id)
		.slice(0, 6)
		.map((task) => ({
			task,
			assignee: pool.find((a) => a.id === task.agentId),
		}))

	const others = pool
		.filter((a) => a.id !== agent.id)
		.map((a) => ({ value: a.id, label: a.name }))

	const openTasks = (id: string) =>
		state.tasks.filter((t) => t.agentId === id && !CLOSED.has(t.status)).length

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
				paused={config.paused}
				busy={update.isPending}
				onMakePrimary={() =>
					setMessage(`${short} est maintenant ton agent principal`)
				}
				onAssign={() => setMessage(`Nouvelle tâche pour ${short}`)}
				onRun={() => setMessage(`Exécution lancée pour ${short}`)}
				onTogglePause={() => {
					updateAgentConfig(agent.id, { paused: !config.paused })
					setMessage(config.paused ? 'Agent relancé' : 'Agent mis en pause')
				}}
				onToggleArchive={toggleArchive}
			/>
			<DemoBanner />
			<AgentTabBar agentId={agent.id} active={tab} />
			<Notice message={message} />
			{tab === 'overview' ? (
				<OverviewTab
					agent={agent}
					form={form}
					skills={config.skills}
					paused={config.paused}
					subCount={subs.length}
					reportsTo={parent?.name ?? DIRECTION}
					tasks={state.tasks.filter((t) => t.agentId === agent.id).slice(0, 6)}
					onCopyKey={() => setMessage('Copié')}
				/>
			) : null}
			{tab === 'instructions' ? (
				<InstructionsTab
					agentId={agent.id}
					mode={mode}
					instr={form.instr}
					onChange={(instr) => patch({ instr })}
					onNotify={setMessage}
				/>
			) : null}
			{tab === 'skills' ? (
				<SkillsTab
					skills={config.skills}
					onToggle={(skill) => {
						const on = !config.skills.includes(skill)
						updateAgentConfig(agent.id, {
							skills: toggled(config.skills, skill),
						})
						setMessage(on ? `${skill} activée` : `${skill} désactivée`)
					}}
				/>
			) : null}
			{tab === 'sources' ? (
				<SourcesTab
					listened={state.sources.filter((s) => s.agentIds.includes(agent.id))}
					available={state.sources.filter(
						(s) => !s.agentIds.includes(agent.id),
					)}
					onDetach={(id) => detachSource(id, agent.id)}
					onAttach={(id) => attachSource(id, agent.id)}
				/>
			) : null}
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
					onAssign={(sub) =>
						setMessage(`Nouvelle tâche pour ${shortName(sub.name)}`)
					}
					onAdd={(id) =>
						send({ reports_to: agent.id }, id, 'Sous-agent ajouté')
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
				<RuntimeTab
					form={form}
					reportsOptions={others}
					testResult={testResult}
					onChange={patch}
					onTest={() =>
						setTestResult(
							`Test réussi : l'agent a répondu « OK » en 2,4 s (${form.model.trim() || 'modèle par défaut'}, ${form.engine}).`,
						)
					}
					onChooseConnection={() => setMessage("Choix d'une connexion gérée")}
					onAdvanced={() =>
						setMessage('Coupe-circuit, fenêtres horaires, concurrence')
					}
				/>
			) : null}
			{tab === 'perms' ? <PermsTab form={form} onChange={patch} /> : null}
			{tab === 'rev' ? (
				<RevisionsTab
					revisions={config.revs}
					onRestore={(label) =>
						setMessage(`${label} restaurée (nouvelle révision créée)`)
					}
				/>
			) : null}

			{showBar ? (
				<SaveBar onCancel={() => setDraft(null)} onSave={save} />
			) : null}
		</Page>
	)
}

function Placeholder({ children }: { children: ReactNode }) {
	return (
		<Page>
			<DemoBanner />
			{children}
		</Page>
	)
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
