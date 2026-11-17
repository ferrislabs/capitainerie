import { useState } from 'react'
import { filterConnectors, parseConnectorTab } from '#/domain/connectors/tabs'
import { ConnectorsView } from '#/domain/connectors/ui/connectors-view'
import {
	agentsWithGrant,
	DEMO_CONNECTORS,
	grantConnector,
	revokeConnector,
	shortName,
	useDemoState,
} from '#/domain/demo'

interface Props {
	tab: string | undefined
}

export function ConnectorsPage({ tab }: Props) {
	const state = useDemoState((s) => s)
	const [notice, setNotice] = useState<string | null>(null)
	const connectors = filterConnectors(parseConnectorTab(tab), DEMO_CONNECTORS)

	return (
		<ConnectorsView
			connectors={connectors}
			agents={state.agents}
			grantedIds={(name) => agentsWithGrant(state, name)}
			notice={notice}
			onConfigure={() => setNotice('Instructions de connexion copiées')}
			onAddAccount={(name) => setNotice(`Ajouter un compte à « ${name} »`)}
			onGrant={(agentId, name) => {
				if (grantConnector(agentId, name)) {
					const agent = state.agents.find((a) => a.id === agentId)
					setNotice(
						`Accès accordé à ${agent ? shortName(agent.name) : agentId}`,
					)
				}
			}}
			onRevoke={(agentId, name) => {
				revokeConnector(agentId, name)
				const agent = state.agents.find((a) => a.id === agentId)
				setNotice(`Accès retiré à ${agent ? shortName(agent.name) : agentId}`)
			}}
		/>
	)
}
