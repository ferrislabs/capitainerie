import { Store } from '@tanstack/store'
import { useSyncExternalStore } from 'react'
import {
	CRITERIA,
	type CriterionId,
	makeCriterion,
	type OnFail,
	type Param,
	type PipelineCriterion,
	PRESETS,
} from '#/domain/sources/criteria'

export interface PipelineConfig {
	criteria: ReadonlyArray<PipelineCriterion>
	destination: string
	action: string
	key: string
	dry: boolean
	dossier: string
}

export const DESTINATIONS = ['Odoo (MCP)', 'HubSpot (MCP)', 'Pipedrive (MCP)']
export const ACTIONS = [
	'Créer un lead',
	'Créer ou mettre à jour une société',
	'Ajouter une note',
]
export const DEDUPE_KEYS = ['SIREN', 'Domaine racine', 'Raison sociale']

function initialConfig(): PipelineConfig {
	return {
		criteria: PRESETS['Prospection B2B'] ?? [],
		destination: 'Odoo (MCP)',
		action: 'Créer un lead',
		key: 'SIREN',
		dry: true,
		dossier: 'iam.acme-logistique.fr',
	}
}

const store = new Store<Readonly<Record<string, PipelineConfig>>>({})

export function resetPipelines(): void {
	store.setState(() => ({}))
}

const configOf = (all: Readonly<Record<string, PipelineConfig>>, id: string) =>
	all[id] ?? initialConfig()

export function usePipeline(sourceId: string): PipelineConfig {
	const stored = useSyncExternalStore(
		(onChange) => store.subscribe(onChange).unsubscribe,
		() => store.state[sourceId],
		() => undefined,
	)
	return stored ?? FALLBACK
}

const FALLBACK = initialConfig()

function patch(
	sourceId: string,
	change: (config: PipelineConfig) => Partial<PipelineConfig>,
) {
	store.setState((all) => {
		const current = configOf(all, sourceId)
		return { ...all, [sourceId]: { ...current, ...change(current) } }
	})
}

export const pipelineActions = {
	toggle: (sourceId: string, index: number) =>
		patch(sourceId, (c) => ({
			criteria: c.criteria.map((x, i) =>
				i === index ? { ...x, enabled: !x.enabled } : x,
			),
		})),
	setParam: (sourceId: string, index: number, param: Param) =>
		patch(sourceId, (c) => ({
			criteria: c.criteria.map((x, i) => (i === index ? { ...x, param } : x)),
		})),
	setOnFail: (sourceId: string, index: number, onFail: OnFail) =>
		patch(sourceId, (c) => ({
			criteria: c.criteria.map((x, i) => (i === index ? { ...x, onFail } : x)),
		})),
	remove: (sourceId: string, index: number) =>
		patch(sourceId, (c) => ({
			criteria: c.criteria.filter((_, i) => i !== index),
		})),
	add: (sourceId: string, id: CriterionId) =>
		patch(sourceId, (c) => ({
			criteria: [...c.criteria, makeCriterion(id, undefined, 'review')],
		})),
	applyPreset: (sourceId: string, name: string) =>
		patch(sourceId, () => ({ criteria: PRESETS[name] ?? [] })),
	set: (sourceId: string, change: Partial<PipelineConfig>) =>
		patch(sourceId, () => change),
}

export const unusedCriteria = (
	criteria: ReadonlyArray<PipelineCriterion>,
): CriterionId[] =>
	(Object.keys(CRITERIA) as CriterionId[]).filter(
		(id) => !criteria.some((c) => c.id === id),
	)
