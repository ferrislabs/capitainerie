import { useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { uniqueAgentId } from '#/domain/agent-team/agent-identity'
import { hasUsableKey, keySourceOf } from '#/domain/agent-team/harness'
import { useHarnesses } from '#/domain/agent-team/hooks/use-harnesses'
import {
	useCreateTeamAgent,
	useTeamAgents,
	useUpdateTeamAgent,
} from '#/domain/agent-team/hooks/use-team-agents'
import { describeAgentError } from '#/domain/agent-team/team-agent'
import { WizardView } from '#/domain/agent-team/ui/wizard-view'
import {
	buildCreateRequest,
	buildWizardPatch,
	canAdvance,
	initialWizard,
	type WizardState,
} from '#/domain/agent-team/wizard'
import { useMcpConnectors } from '#/domain/connectors/hooks/use-mcp-connectors'
import { useGithubRepositoryChoices } from '#/domain/integrations/hooks/use-github'
import {
	useMyProviders,
	useOrgProviders,
	usePutMyProvider,
} from '#/domain/providers/hooks/use-providers'
import { useSkillCatalog } from '#/domain/skills/hooks/use-skills'

export function AgentWizardPage() {
	const [state, setState] = useState<WizardState>(initialWizard)
	const team = useTeamAgents(false)
	const create = useCreateTeamAgent()
	const configure = useUpdateTeamAgent()
	const connectorsQuery = useMcpConnectors(false)
	const catalog = useSkillCatalog()
	const repositories = useGithubRepositoryChoices()
	const harnessesQuery = useHarnesses()
	const mine = useMyProviders()
	const org = useOrgProviders()
	const saveKey = usePutMyProvider()
	const navigate = useNavigate()
	const harnesses = harnessesQuery.data?.harnesses ?? []
	const harness = harnesses.find((h) => h.id === state.adapter)
	const orgKeys = org.isError
		? 'forbidden'
		: org.isPending
			? undefined
			: org.data.providers
	const keySource = harness
		? keySourceOf(harness.provider, mine.data, orgKeys)
		: ({ kind: 'loading' } as const)
	const keyAvailable = hasUsableKey(keySource)
	const agents = team.data?.items ?? []
	const connectors = (connectorsQuery.data?.items ?? []).map((c) => ({
		slug: c.slug,
		name: c.name,
	}))
	const skillOptions = [
		...catalog,
		...state.skills.filter((s) => !catalog.includes(s)),
	]
	const pending = create.isPending || configure.isPending || saveKey.isPending

	const patch = (change: Partial<WizardState>) =>
		setState((current) => ({ ...current, ...change }))

	const createAgent = () => {
		const slug = uniqueAgentId(
			state.name,
			agents.map((a) => a.slug),
		)
		create.mutate(
			{
				body: buildCreateRequest(
					state,
					slug,
					connectors.map((c) => c.slug),
				),
			},
			{
				onSuccess: (agent) => {
					const open = () =>
						void navigate({
							to: '/agents/$agentId/{-$tab}',
							params: { agentId: agent.id },
						})
					configure.mutate(
						{
							path: { agent_id: agent.id },
							body: buildWizardPatch(state, agent, harness?.name),
						},
						{ onSettled: open },
					)
				},
			},
		)
	}

	const submit = () => {
		const typed = state.key.trim()
		if (!harness || !typed) {
			createAgent()
			return
		}
		saveKey.mutate(
			{
				path: { kind: harness.provider },
				body: {
					api_key: typed,
					default_model: state.model,
					is_default: false,
				},
			},
			{ onSuccess: createAgent },
		)
	}

	return (
		<Page>
			{saveKey.isError ? (
				<InlineAlert tone="danger" className="mb-2.5">
					Impossible d'enregistrer ta clé API. Vérifie-la puis réessaie.
				</InlineAlert>
			) : null}
			{create.isError ? (
				<InlineAlert tone="danger" className="mb-2.5">
					{describeAgentError(create.error)}
				</InlineAlert>
			) : null}
			<WizardView
				state={state}
				reportsOptions={agents.map((a) => ({ value: a.id, label: a.name }))}
				connectors={connectors}
				connectorsLoading={connectorsQuery.isPending}
				repositories={repositories}
				harnesses={harnesses}
				harnessesStatus={
					harnessesQuery.isError
						? 'error'
						: harnessesQuery.isPending
							? 'pending'
							: 'success'
				}
				keySource={keySource}
				skillOptions={skillOptions}
				canAdvance={canAdvance(state, keyAvailable) && !pending}
				onChange={patch}
				onBack={() => patch({ step: Math.max(0, state.step - 1) })}
				onNext={() =>
					canAdvance(state, keyAvailable) && patch({ step: state.step + 1 })
				}
				onCreate={() => {
					if (!pending) submit()
				}}
			/>
		</Page>
	)
}
