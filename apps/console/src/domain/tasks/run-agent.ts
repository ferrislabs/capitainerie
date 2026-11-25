import type { Schemas } from '#/api/api.client'

export type RunStatus = Schemas.RunStatusResponse

export const RUN_STATUS_LABEL: Record<RunStatus, string> = {
	queued: 'En file d’attente',
	running: 'En cours',
	succeeded: 'Terminée',
	failed: 'Échec',
	cancelled: 'Annulée',
}

export const RUN_POLL_MS = 2000

export function isActiveRun(status: RunStatus): boolean {
	return status === 'queued' || status === 'running'
}

/**
 * Un agent assigné suffit : il travaille avec ses connecteurs MCP ou sur l'un
 * de ses dépôts GitHub. Le serveur refuse un agent qui n'a ni l'un ni l'autre.
 */
export function canRunAgent(column: string, hasAgent: boolean): boolean {
	return hasAgent && column !== 'done' && column !== 'rejected'
}

export type RunTarget = 'connectors' | 'repository'

export interface RunPlan {
	target: RunTarget
	repository: string
	baseBranch: string
	allowWrites: boolean
}

/** Le dépôt accordé que le titre ou la description de la tâche nomme, s'il y en a un. */
export function mentionedRepository(
	grants: ReadonlyArray<string>,
	text: string,
): string | undefined {
	const haystack = text.toLowerCase()
	return [...grants]
		.sort((a, b) => b.length - a.length)
		.find((grant) => {
			const needle = grant.toLowerCase()
			let at = haystack.indexOf(needle)
			while (at !== -1) {
				const before = haystack[at - 1]
				const end = at + needle.length
				const after = haystack[end] === '.' ? haystack[end + 1] : haystack[end]
				if (!isNameChar(before) && !isNameChar(after)) return true
				at = haystack.indexOf(needle, at + 1)
			}
			return false
		})
}

function isNameChar(char: string | undefined): boolean {
	return char !== undefined && /[a-z0-9_.-]/.test(char)
}

export function initialRunPlan(
	grants: ReadonlyArray<string>,
	hasConnectors: boolean,
	text: string,
): RunPlan {
	const mentioned = mentionedRepository(grants, text)
	const target: RunTarget =
		grants.length > 0 && (mentioned !== undefined || !hasConnectors)
			? 'repository'
			: 'connectors'
	return {
		target,
		repository: mentioned ?? grants[0] ?? '',
		baseBranch: '',
		allowWrites: false,
	}
}

export function runRequest(
	plan: RunPlan,
	version: number,
): Schemas.RunAgentRequest {
	if (plan.target === 'connectors' || plan.repository === '') {
		return { expected_version: version, allow_writes: plan.allowWrites }
	}
	const baseBranch = plan.baseBranch.trim()
	return {
		expected_version: version,
		allow_writes: plan.allowWrites,
		repository: plan.repository,
		...(baseBranch ? { base_branch: baseBranch } : {}),
	}
}

export function writesHint(plan: RunPlan): string {
	if (plan.target === 'connectors') {
		return 'Les écritures modifient les données du service connecté.'
	}
	return plan.allowWrites
		? "L'agent pousse une branche et ouvre une PR brouillon."
		: "Audit en lecture seule : l'agent lit le code et produit un rapport."
}

export type RunErrorKind =
	| 'not_connected'
	| 'no_provider'
	| 'github'
	| 'agent_repositories'
	| 'message'

export interface RunErrorInfo {
	kind: RunErrorKind
	message: string
	stale: boolean
}

function fieldOf(error: unknown): {
	status?: number
	code?: string
	message?: string
} {
	return (error ?? {}) as { status?: number; code?: string; message?: string }
}

/** Le serveur range la raison dans `code` ou, pour un conflit générique, dans `message`. */
function keyed(
	code: string | undefined,
	message: string | undefined,
	key: string,
): boolean {
	return code === key || message === key
}

function reasonOf(message: string | undefined): string | undefined {
	if (!message) return undefined
	const at = message.indexOf(' : ')
	const reason = at === -1 ? message : message.slice(at + 3)
	return reason.trim() || undefined
}

export function describeRunError(error: unknown): RunErrorInfo {
	const { status, code, message } = fieldOf(error)
	const is = (key: string) => keyed(code, message, key)
	const info = (
		text: string,
		stale = false,
		kind: RunErrorKind = 'message',
	): RunErrorInfo => ({ kind, message: text, stale })
	if (status === 409) {
		if (is('not_connected')) {
			return info(
				message ?? "Un connecteur de l'agent n'est pas connecté.",
				false,
				'not_connected',
			)
		}
		if (is('no_provider')) {
			return info(
				message ??
					'Aucun fournisseur LLM connecté. Connectez-en un pour lancer cet agent.',
				false,
				'no_provider',
			)
		}
		if (is('github_not_connected')) {
			return info(
				"GitHub n'est pas connecté à l'organisation : connecte-le avant de lancer l'agent sur un dépôt.",
				false,
				'github',
			)
		}
		if (is('executor_disabled')) {
			const reason = reasonOf(message)
			return info(
				`Les exécutions sur un dépôt sont désactivées sur ce serveur${reason ? ` : ${reason}` : ''}.`,
			)
		}
		if (is('agent_identity_not_active')) {
			return info(
				"L'identité FerrisKey de l'agent n'est pas encore prête : réessaie dans une minute.",
			)
		}
		if (is('execution_already_active')) {
			return info('Une exécution est déjà en cours pour cette tâche.')
		}
		return info('Cette tâche a changé entre-temps. Réessaie.', true)
	}
	if (status === 422) {
		switch (code) {
			case 'card_without_agent':
				return info("Cette tâche n'a pas d'agent assigné.")
			case 'agent_archived':
				return info("L'agent assigné est archivé : assigne-en un autre.")
			case 'agent_paused':
				return info(
					"L'agent assigné est en pause : reprends-le depuis l'équipe avant de le lancer.",
				)
			case 'agent_without_connector':
				return info(
					"L'agent n'a aucun connecteur autorisé : accorde-lui en un, ou choisis un de ses dépôts GitHub.",
					false,
					'agent_repositories',
				)
			case 'repository_not_granted':
				return info(
					"L'agent n'a pas accès à ce dépôt : ajoute-le à ses dépôts GitHub.",
					false,
					'agent_repositories',
				)
			case 'invalid':
				return info(
					'Le dépôt ou la branche de base est invalide : vérifie le nom de la branche.',
				)
			default:
				return info("Cette tâche ne peut pas être exécutée par l'agent.")
		}
	}
	if (status === 403) {
		if (is('repository_not_allowed')) {
			return info(
				"L'application GitHub n'a pas accès à ce dépôt : ajoute-le à l'installation sur GitHub.",
				false,
				'github',
			)
		}
		return info("Tu n'as pas le droit d'exécuter cet agent.")
	}
	return info("Le lancement de l'agent a échoué. Réessaie.")
}
