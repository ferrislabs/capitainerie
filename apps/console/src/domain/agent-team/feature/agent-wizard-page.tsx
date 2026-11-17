import { useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { seedAgentConfig } from '#/domain/agent-team/agent-config-store'
import { uniqueAgentId } from '#/domain/agent-team/agent-identity'
import { WizardView } from '#/domain/agent-team/ui/wizard-view'
import {
	buildNewAgent,
	buildWizardConfig,
	canAdvance,
	initialWizard,
	type WizardState,
} from '#/domain/agent-team/wizard'
import {
	addAgent,
	attachSource,
	DEMO_CONNECTORS,
	grantConnector,
	shortName,
	useDemoState,
} from '#/domain/demo'

const CONNECTABLE = DEMO_CONNECTORS.filter((c) => c.accounts.length > 0).map(
	(c) => c.name,
)

export function AgentWizardPage() {
	const [state, setState] = useState<WizardState>(initialWizard)
	const agents = useDemoState((s) => s.agents)
	const sources = useDemoState((s) => s.sources)
	const navigate = useNavigate()

	const patch = (change: Partial<WizardState>) =>
		setState((current) => ({ ...current, ...change }))

	const create = () => {
		const id = uniqueAgentId(
			state.name,
			agents.map((a) => a.id),
		)
		const agent = buildNewAgent(state, id)
		addAgent(agent)
		seedAgentConfig(id, buildWizardConfig(state, agent))
		for (const connector of state.connectors) grantConnector(id, connector)
		for (const sourceId of state.sources) attachSource(sourceId, id)
		void navigate({
			to: '/agents/$agentId/{-$tab}',
			params: { agentId: id },
		})
	}

	return (
		<Page>
			<PageHeader
				title="Nouvel agent"
				description="Cinq étapes : nom, adaptateur, connexion, configuration, confirmation."
			/>
			<DemoBanner />
			<WizardView
				state={state}
				reportsOptions={agents.map((a) => shortName(a.name))}
				sources={sources}
				connectors={CONNECTABLE}
				canAdvance={canAdvance(state)}
				onChange={patch}
				onBack={() => patch({ step: Math.max(0, state.step - 1) })}
				onNext={() => canAdvance(state) && patch({ step: state.step + 1 })}
				onCreate={create}
			/>
		</Page>
	)
}
