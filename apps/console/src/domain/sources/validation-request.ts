import type { DemoApproval } from '#/domain/demo'

interface Input {
	approvals: ReadonlyArray<DemoApproval>
	raison: string
	sourceName: string
	reasons: ReadonlyArray<string>
	requestedBy: string | null
}

export function validationTitle(raison: string, sourceName: string): string {
	return `Valider le rapprochement : ${raison} (${sourceName})`
}

export function nextApprovalId(approvals: ReadonlyArray<DemoApproval>): string {
	return `APP-${approvals.length + 1}`
}

/** `null` quand une demande existe déjà pour ce dossier et cette source. */
export function buildValidationRequest({
	approvals,
	raison,
	sourceName,
	reasons,
	requestedBy,
}: Input): DemoApproval | null {
	const title = validationTitle(raison, sourceName)
	if (approvals.some((a) => a.title === title)) return null
	return {
		id: nextApprovalId(approvals),
		title,
		detail: reasons.join(' ; '),
		requestedBy,
		status: 'pending',
		at: "à l'instant",
		decidedBy: '',
		taskId: undefined,
	}
}
