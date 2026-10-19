import type { Schemas } from '#/api/api.client'
import type { ModelAssignments } from '#/domain/settings/roles'

export interface SettingState {
	label: string
	ready: boolean
}

export function providersState(
	providers: ReadonlyArray<Schemas.ProviderResponse>,
): SettingState {
	if (providers.length === 0) {
		return { label: 'Aucun fournisseur', ready: false }
	}
	return {
		label: `${providers.length} ${providers.length > 1 ? 'fournisseurs' : 'fournisseur'}`,
		ready: true,
	}
}

export function modelsState(
	assignments: ModelAssignments | null | undefined,
): SettingState {
	if (!assignments) return { label: 'Modèles non assignés', ready: false }
	if (!assignments.transcriber) {
		return { label: 'Transcripteur non choisi', ready: false }
	}
	if (!assignments.reader) {
		return { label: 'Lecteur de PDF non choisi', ready: false }
	}
	return { label: 'Modèles assignés', ready: true }
}

export function integrationsState(
	github: Schemas.GithubIntegrationResponse,
): SettingState {
	if (github.installation) return { label: 'GitHub connecté', ready: true }
	if (github.app) return { label: 'GitHub à connecter', ready: false }
	return { label: 'GitHub non configuré', ready: false }
}
