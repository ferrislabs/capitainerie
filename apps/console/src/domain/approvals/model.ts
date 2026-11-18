import type { Schemas } from '#/api/api.client'
import type { SevTone } from '#/domain/audit/ui/sev'

export type Approval = Schemas.ApprovalResponse
export type ApprovalStatus = Schemas.ApprovalStatusDto

export const APPROVAL_TABS: ReadonlyArray<{
	status: ApprovalStatus
	label: string
}> = [
	{ status: 'pending', label: 'En attente' },
	{ status: 'approved', label: 'Approuvées' },
	{ status: 'rejected', label: 'Refusées' },
	{ status: 'expired', label: 'Expirées' },
]

export const STATUS_LABEL: Record<ApprovalStatus, string> = {
	pending: 'En attente',
	approved: 'Approuvée',
	rejected: 'Refusée',
	expired: 'Expirée',
}

export const STATUS_TONE: Record<ApprovalStatus, SevTone> = {
	pending: 'warn',
	approved: 'ok',
	rejected: 'crit',
	expired: 'info',
}

export const KIND_LABEL: Record<Schemas.ApprovalKindDto, string> = {
	tool_call: "Appel d'outil",
	agent_action: "Action d'agent",
}

export interface ToolCall {
	tool: string
	arguments: unknown
}

/** Le détail d'un `tool_call` est `{ tool, arguments }` ; `null` sinon. */
export function toolCallOf(approval: Approval): ToolCall | null {
	if (approval.kind !== 'tool_call') return null
	const { tool, arguments: args } = approval.detail
	if (typeof tool !== 'string') return null
	return { tool, arguments: args ?? {} }
}

export function prettyJson(value: unknown): string {
	return JSON.stringify(value, null, 2) ?? String(value)
}

export function formatWhen(iso: string): string {
	const date = new Date(iso)
	if (Number.isNaN(date.getTime())) return iso
	return date.toLocaleString('fr-FR', {
		day: 'numeric',
		month: 'short',
		hour: '2-digit',
		minute: '2-digit',
	})
}

export function describeDecideError(error: unknown): string {
	const status = (error as { status?: number } | null)?.status
	if (status === 409)
		return "Cette demande n'est plus en attente : elle a déjà été traitée ou a expiré. La liste est à jour."
	if (status === 403) return "Tu n'as pas le droit de décider de cette demande."
	if (status === 404) return "Cette demande n'existe plus."
	return 'La décision a échoué. Réessaie.'
}
