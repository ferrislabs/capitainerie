import {
	type AgentForm,
	buildForm,
	describeChanges,
	diffForm,
	type StoreField,
} from '#/domain/agent-team/agent-config'
import {
	getAgentConfig,
	updateAgentConfig,
} from '#/domain/agent-team/agent-config-store'
import { DIRECTION } from '#/domain/agent-team/constants'
import { demoStore, setReportsTo, shortName, updateAgent } from '#/domain/demo'

export function readAgentForm(agentId: string): AgentForm {
	const agent = demoStore.state.agents.find((a) => a.id === agentId)
	if (!agent) throw new Error(`Agent inconnu : ${agentId}`)
	return buildForm(
		agent,
		demoStore.state.reportsTo[agentId] ?? null,
		getAgentConfig(agentId),
	)
}

const STORE_FIELDS: ReadonlyArray<StoreField> = [
	'name',
	'role',
	'model',
	'reports',
]

/** Enregistre le formulaire et renvoie les champs modifiés ; une révision est ajoutée s'il y en a. */
export function saveAgentForm(
	agentId: string,
	next: AgentForm,
): Array<keyof AgentForm> {
	const base = readAgentForm(agentId)
	const name = next.name.trim() || base.name
	const form = { ...next, name }
	const changed = diffForm(base, form)
	if (changed.length === 0) return changed

	const previousShort = shortName(base.name)
	updateAgent(agentId, { name, role: form.role, model: form.model })
	if (name !== base.name) {
		for (const [id, parent] of Object.entries(demoStore.state.reportsTo)) {
			if (parent === previousShort) setReportsTo(id, shortName(name))
		}
	}
	setReportsTo(agentId, form.reports === DIRECTION ? null : form.reports)

	const fields = { ...form } as Partial<AgentForm>
	for (const key of STORE_FIELDS) delete fields[key]
	const config = getAgentConfig(agentId)
	const rev = config.rev + 1
	updateAgentConfig(agentId, {
		fields: { ...config.fields, ...(fields as AgentConfigFields) },
		rev,
		revs: [
			{
				label: `Rév. ${rev}`,
				author: 'Baptiste Parmantier',
				when: "à l'instant",
				changed: describeChanges(changed),
			},
			...config.revs,
		],
	})
	return changed
}

type AgentConfigFields = ReturnType<typeof getAgentConfig>['fields']
