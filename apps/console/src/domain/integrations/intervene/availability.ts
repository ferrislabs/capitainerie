import type { Schemas } from '#/api/api.client'
import {
	activeInstances,
	type Instance,
} from '#/domain/integrations/intervene/grants'

export type InterveneAvailability =
	| { kind: 'loading' }
	| { kind: 'ready' }
	| { kind: 'blocked'; reason: string }

export const NO_CLIENT_REASON = 'Rattachez d’abord la carte à un client'
export const NO_INSTANCE_REASON = 'Ce client n’a aucune intégration'

export function interveneAvailability(
	client: Schemas.ClientRefResponse | null | undefined,
	instances: ReadonlyArray<Instance> | undefined,
): InterveneAvailability {
	if (!client) return { kind: 'blocked', reason: NO_CLIENT_REASON }
	if (instances === undefined) return { kind: 'loading' }
	return activeInstances(instances).length === 0
		? { kind: 'blocked', reason: NO_INSTANCE_REASON }
		: { kind: 'ready' }
}
