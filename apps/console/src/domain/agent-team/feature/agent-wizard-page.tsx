import { useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Page } from '#/components/layout/page'
import { seedAgentConfig } from '#/domain/agent-team/agent-config-store'
import { uniqueAgentId } from '#/domain/agent-team/agent-identity'
import {
	useCreateTeamAgent,
	useTeamAgents,
} from '#/domain/agent-team/hooks/use-team-agents'
import { describeAgentError } from '#/domain/agent-team/team-agent'
import { WizardView } from '#/domain/agent-team/ui/wizard-view'
import {
	buildCreateRequest,
	buildWizardConfig,
	canAdvance,
	initialWizard,
	type WizardState,
} from '#/domain/agent-team/wizard'
import { attachSource, DEMO_CONNECTORS, useDemoState } from '#/domain/demo'

const CONNECTABLE = DEMO_CONNECTORS.filter((c) => c.accounts.length > 0).map(
	(c) => c.name,
)

export function AgentWizardPage() {
	const [state, setState] = useState<WizardState>(initialWizard)
	const team = useTeamAgents(false)
	const create = useCreateTeamAgent()
	const sources = useDemoState((s) => s.sources)
	const navigate = useNavigate()
	const agents = team.data?.items ?? []

	const patch = (change: Partial<WizardState>) =>
		setState((current) => ({ ...current, ...change }))

	const submit = () => {
		const slug = uniqueAgentId(
			state.name,
			agents.map((a) => a.slug),
		)
		create.mutate(
			{ body: buildCreateRequest(state, slug, CONNECTABLE) },
			{
				onSuccess: (agent) => {
					seedAgentConfig(agent.id, buildWizardConfig(state, agent))
					for (const sourceId of state.sources) attachSource(sourceId, agent.id)
					void navigate({
						to: '/agents/$agentId/{-$tab}',
						params: { agentId: agent.id },
					})
				},
			},
		)
	}

	return (
		<Page>
			<DemoBanner />
			{create.isError ? (
				<InlineAlert tone="danger" className="mb-2.5">
					{describeAgentError(create.error)}
				</InlineAlert>
			) : null}
			<WizardView
				state={state}
				reportsOptions={agents.map((a) => ({ value: a.id, label: a.name }))}
				sources={sources}
				connectors={CONNECTABLE}
				canAdvance={canAdvance(state) && !create.isPending}
				onChange={patch}
				onBack={() => patch({ step: Math.max(0, state.step - 1) })}
				onNext={() => canAdvance(state) && patch({ step: state.step + 1 })}
				onCreate={() => {
					if (!create.isPending) submit()
				}}
			/>
		</Page>
	)
}
