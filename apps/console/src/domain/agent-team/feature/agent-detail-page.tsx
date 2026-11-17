import { Link } from '@tanstack/react-router'
import { UserX } from 'lucide-react'
import { useState } from 'react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { EmptyState } from '#/components/layout/empty-state'
import { Notice, useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { Button } from '#/components/ui/button'
import {
	type AgentForm,
	buildForm,
	diffForm,
} from '#/domain/agent-team/agent-config'
import {
	getAgentConfig,
	updateAgentConfig,
	useAgentConfig,
} from '#/domain/agent-team/agent-config-store'
import {
	type AgentTabId,
	resolveAgentTab,
} from '#/domain/agent-team/agent-tabs'
import { HEAD_OF_TEAM } from '#/domain/agent-team/constants'
import { SECRETS } from '#/domain/agent-team/mock/agent-fixtures'
import { saveAgentForm } from '#/domain/agent-team/save-agent'
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
import { KeysTab } from '#/domain/agent-team/ui/keys-tab'
import { OverviewTab } from '#/domain/agent-team/ui/overview-tab'
import { PermsTab } from '#/domain/agent-team/ui/perms-tab'
import { RevisionsTab } from '#/domain/agent-team/ui/revisions-tab'
import { RuntimeTab } from '#/domain/agent-team/ui/runtime-tab'
import { SaveBar } from '#/domain/agent-team/ui/save-bar'
import { SecretsTab } from '#/domain/agent-team/ui/secrets-tab'
import { SkillsTab } from '#/domain/agent-team/ui/skills-tab'
import { SourcesTab } from '#/domain/agent-team/ui/sources-tab'
import { toggled } from '#/domain/agent-team/wizard'
import {
	ancestorIds,
	attachSource,
	DEMO_CONNECTORS,
	type DemoAgent,
	detachSource,
	grantConnector,
	revokeConnector,
	setReportsTo,
	shortName,
	subAgentIds,
	useDemoState,
} from '#/domain/demo'

interface Props {
	agentId: string
	tab?: string
	mode?: InstructionsMode
}

interface BodyProps {
	agent: DemoAgent
	tab: AgentTabId
	mode: InstructionsMode
}

const CLOSED = new Set(['done', 'cancelled'])
const SAVE_TABS: ReadonlyArray<AgentTabId> = [
	'instructions',
	'team',
	'connectors',
	'runtime',
	'secrets',
	'perms',
]

function AgentDetailBody({ agent, tab, mode }: BodyProps) {
	const state = useDemoState((s) => s)
	const config = useAgentConfig(agent.id)
	const [draft, setDraft] = useState<AgentForm | null>(null)
	const [message, setMessage] = useNotice()
	const [testResult, setTestResult] = useState<string>()

	const base = buildForm(agent, state.reportsTo[agent.id] ?? null, config)
	const form = draft ?? base
	const dirty = diffForm(base, form).length > 0
	const patch = (change: Partial<AgentForm>) => setDraft({ ...form, ...change })
	const short = shortName(agent.name)

	const byId = (id: string) => state.agents.find((a) => a.id === id)
	const subIds = subAgentIds(state, agent.id)
	const ancestors = ancestorIds(state, agent.id)
	const granted = state.grants[agent.id] ?? []

	const save = () => {
		saveAgentForm(agent.id, form)
		setDraft(null)
		setMessage(`Enregistré · révision ${getAgentConfig(agent.id).rev} créée`)
	}

	const toggleConnector = (name: string, allowed: boolean) => {
		if (!allowed) {
			revokeConnector(agent.id, name)
			setMessage(`Accès retiré à ${short}`)
		} else if (grantConnector(agent.id, name)) {
			setMessage(`${short} peut utiliser ${name}`)
		} else {
			setMessage(`Connecte d'abord un compte à « ${name} »`)
		}
	}

	const removeSub = (subId: string) =>
		setReportsTo(subId, short === HEAD_OF_TEAM ? null : HEAD_OF_TEAM)

	const taskOf = (id: string) => state.tasks.find((t) => t.id === id)
	const recent: DelegatedTask[] = state.tasks
		.filter((t) => t.parentId && taskOf(t.parentId)?.agentId === agent.id)
		.slice(0, 6)
		.map((task) => ({ task, assignee: byId(task.agentId) }))

	const others = state.agents
		.filter((a) => a.id !== agent.id)
		.map((a) => shortName(a.name))

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
				onMakePrimary={() =>
					setMessage(`${short} est maintenant ton agent principal`)
				}
				onAssign={() => setMessage(`Nouvelle tâche pour ${short}`)}
				onRun={() => setMessage(`Exécution lancée pour ${short}`)}
				onTogglePause={() => {
					updateAgentConfig(agent.id, { paused: !config.paused })
					setMessage(config.paused ? `${short} relancé` : `${short} en pause`)
				}}
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
					subCount={subIds.length}
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
					ancestors={ancestors.flatMap((id) => byId(id) ?? [])}
					subs={subIds.flatMap((id) => {
						const sub = byId(id)
						return sub ? [{ agent: sub, openTasks: openTasks(id) }] : []
					})}
					candidates={state.agents.filter(
						(a) =>
							a.id !== agent.id &&
							!subIds.includes(a.id) &&
							!ancestors.includes(a.id),
					)}
					recent={recent}
					form={form}
					onChange={patch}
					onRemove={removeSub}
					onAdd={(id) => setReportsTo(id, short)}
				/>
			) : null}
			{tab === 'connectors' ? (
				<ConnectorsTab
					connectors={DEMO_CONNECTORS}
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
						setTestResult("Test réussi : l'agent a répondu en 2,4 s.")
					}
					onChooseConnection={() => setMessage("Choix d'une connexion gérée")}
				/>
			) : null}
			{tab === 'secrets' ? (
				<SecretsTab
					env={form.env}
					secrets={SECRETS}
					onChange={(env) => patch({ env })}
				/>
			) : null}
			{tab === 'perms' ? <PermsTab form={form} onChange={patch} /> : null}
			{tab === 'keys' ? (
				<KeysTab
					keys={config.keys}
					onCreate={() => {
						updateAgentConfig(agent.id, {
							keys: [
								...config.keys,
								{
									name: 'Nouvelle clé',
									preview: `pcp_${(0x9a1f + config.keys.length * 0x111).toString(16)}…e2c7`,
									created: "à l'instant",
									lastUsed: 'jamais',
								},
							],
						})
						setMessage('Clé créée')
					}}
					onRevoke={(index) =>
						updateAgentConfig(agent.id, {
							keys: config.keys.filter((_, i) => i !== index),
						})
					}
				/>
			) : null}
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

export function AgentDetailPage({ agentId, tab, mode = 'lire' }: Props) {
	const agent = useDemoState((s) => s.agents.find((a) => a.id === agentId))
	if (!agent) {
		return (
			<Page>
				<DemoBanner />
				<EmptyState
					icon={UserX}
					title="Agent introuvable"
					description="Cet agent n'existe pas ou n'a pas été créé pendant cette session."
					action={
						<Button asChild variant="outline">
							<Link to="/agents/equipe">Retour à l'équipe</Link>
						</Button>
					}
				/>
			</Page>
		)
	}
	return (
		<AgentDetailBody
			key={agent.id}
			agent={agent}
			tab={resolveAgentTab(tab)}
			mode={mode}
		/>
	)
}
