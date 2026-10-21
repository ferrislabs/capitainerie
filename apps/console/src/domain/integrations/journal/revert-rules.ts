import type { Schemas } from '#/api/api.client'

export function canRevert(action: Schemas.IntegrationActionResponse): boolean {
	return (
		action.revertible &&
		action.outcome === 'succeeded' &&
		!action.reverted_by_action_id &&
		!action.reverts_action_id
	)
}
