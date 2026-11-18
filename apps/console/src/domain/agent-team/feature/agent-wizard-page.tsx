import { useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { uniqueAgentId } from '#/domain/agent-team/agent-identity'
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
import { useSkillCatalog } from '#/domain/skills/hooks/use-skills'

export function AgentWizardPage() {
	const [state, setState] = useState<WizardState>(initialWizard)
	const team = useTeamAgents(false)
	const create = useCreateTeamAgent()
	const configure = useUpdateTeamAgent()
	const connectorsQuery = useMcpConnectors(false)
	const catalog = useSkillCatalog()
	const navigate = useNavigate()
	const agents = team.data?.items ?? []
	const connectors = (connectorsQuery.data?.items ?? []).map((c) => ({
		slug: c.slug,
		name: c.name,
	}))
	const skillOptions = [
		...catalog,
		...state.skills.filter((s) => !catalog.includes(s)),
	]
	const pending = create.isPending || configure.isPending

	const patch = (change: Partial<WizardState>) =>
		setState((current) => ({ ...current, ...change }))

	const submit = () => {
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
							body: buildWizardPatch(state, agent),
						},
						{ onSettled: open },
					)
				},
			},
		)
	}

	return (
		<Page>
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
				skillOptions={skillOptions}
				canAdvance={canAdvance(state) && !pending}
				onChange={patch}
				onBack={() => patch({ step: Math.max(0, state.step - 1) })}
				onNext={() => canAdvance(state) && patch({ step: state.step + 1 })}
				onCreate={() => {
					if (!pending) submit()
				}}
			/>
		</Page>
	)
}
