import { describe, expect, it } from 'vitest'
import {
	integrationsState,
	modelsState,
	providersState,
} from '#/domain/settings/overview'
import {
	ANTHROPIC_PROVIDER,
	COMPATIBLE_PROVIDER,
} from '#/domain/settings/test-fixtures'

const ASSIGNMENTS = {
	extractor: { provider: 'anthropic', model: 'a' },
	judge: { provider: 'anthropic', model: 'a' },
	writer: { provider: 'anthropic', model: 'a' },
	embeddings: { provider: 'mistral', model: 'e' },
} as const

describe('providersState', () => {
	it('counts providers', () => {
		expect(providersState([])).toEqual({
			label: 'Aucun fournisseur',
			ready: false,
		})
		expect(providersState([ANTHROPIC_PROVIDER])).toEqual({
			label: '1 fournisseur',
			ready: true,
		})
		expect(providersState([ANTHROPIC_PROVIDER, COMPATIBLE_PROVIDER])).toEqual({
			label: '2 fournisseurs',
			ready: true,
		})
	})
})

describe('modelsState', () => {
	it('asks for assignments when none are saved', () => {
		expect(modelsState(null)).toEqual({
			label: 'Modèles non assignés',
			ready: false,
		})
	})

	it('flags the optional transcriber and reader', () => {
		expect(modelsState(ASSIGNMENTS)).toEqual({
			label: 'Transcripteur non choisi',
			ready: false,
		})
		expect(
			modelsState({
				...ASSIGNMENTS,
				transcriber: { provider: 'mistral', model: 't' },
			}),
		).toEqual({ label: 'Lecteur de PDF non choisi', ready: false })
	})

	it('is ready once every role has a model', () => {
		expect(
			modelsState({
				...ASSIGNMENTS,
				transcriber: { provider: 'mistral', model: 't' },
				reader: { provider: 'mistral', model: 'r' },
			}),
		).toEqual({ label: 'Modèles assignés', ready: true })
	})
})

describe('integrationsState', () => {
	it('tells GitHub apart in its three states', () => {
		expect(integrationsState({})).toEqual({
			label: 'GitHub non configuré',
			ready: false,
		})
		expect(
			integrationsState({
				app: {
					slug: 'cap',
					html_url: 'https://github.com/apps/cap',
					owner_login: 'ada',
					owner_type: 'User',
					created_at: '2026-10-16T20:05:19Z',
					created_by: 'ada',
				},
			}),
		).toEqual({ label: 'GitHub à connecter', ready: false })
		expect(
			integrationsState({
				installation: {
					account_login: 'ada',
					connected_at: '2026-10-16T20:05:19Z',
					connected_by: 'ada',
					installation_id: 1,
				},
			}),
		).toEqual({ label: 'GitHub connecté', ready: true })
	})
})
