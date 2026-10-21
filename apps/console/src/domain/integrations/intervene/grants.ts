import type { Schemas } from '#/api/api.client'

export type Instance = Schemas.IntegrationInstanceResponse
export type GrantChoice = 'none' | Schemas.GrantAccessDto
export type GrantChoices = Readonly<Record<string, GrantChoice | undefined>>

export const INSTRUCTIONS_MAX_CHARS = 4000

export interface InterveneForm {
	choices: GrantChoices
	instructions: string
	confirmed: boolean
}

export function instructionsLength(text: string): number {
	return [...text].length
}

export function activeInstances(
	instances: ReadonlyArray<Instance>,
): Array<Instance> {
	return instances.filter((instance) => instance.archived_at == null)
}

export function needsProductionConfirmation(
	instances: ReadonlyArray<Instance>,
	choices: GrantChoices,
): boolean {
	return instances.some(
		(instance) =>
			instance.environment === 'production' && choices[instance.id] === 'write',
	)
}

function grantsOf(
	instances: ReadonlyArray<Instance>,
	choices: GrantChoices,
): Array<Schemas.InstanceGrantRequest> {
	return instances.flatMap((instance) => {
		const access = choices[instance.id]
		return access === 'read' || access === 'write'
			? [{ instance_id: instance.id, access }]
			: []
	})
}

export function canSubmitIntervention(
	instances: ReadonlyArray<Instance>,
	form: InterveneForm,
): boolean {
	return (
		grantsOf(instances, form.choices).length > 0 &&
		(!needsProductionConfirmation(instances, form.choices) || form.confirmed) &&
		instructionsLength(form.instructions) <= INSTRUCTIONS_MAX_CHARS
	)
}

export function interveneBody(
	instances: ReadonlyArray<Instance>,
	form: InterveneForm,
	expectedVersion: number,
): Schemas.InterveneCardRequest {
	const instructions = form.instructions.trim()
	return {
		grants: grantsOf(instances, form.choices),
		expected_version: expectedVersion,
		...(instructions === '' ? {} : { instructions }),
		...(form.confirmed && needsProductionConfirmation(instances, form.choices)
			? { confirm_production_writes: true }
			: {}),
	}
}
