export interface Dossier {
	raison: string
	siren: string
	indices: number
	active: boolean
	collective: boolean
	effectif: number | null
	naf: string
	pays: string
	crm: boolean
	rdapMasque?: boolean
	age: number
	task: string
}

export type CriterionResult = 'ok' | 'ko' | 'unknown'
export type OnFail = 'reject' | 'review' | 'ignore'
export type Verdict = 'qualified' | 'review' | 'rejected'
export type CriterionId =
	| 'siren'
	| 'active'
	| 'collective'
	| 'effectif'
	| 'naf'
	| 'pays'
	| 'crm'
	| 'rdap'
	| 'age'
export type Param = string | number | undefined

interface CriterionDef {
	label: string
	defaultParam?: string | number
	unit?: string
	evaluate: (dossier: Dossier, param: Param) => CriterionResult
}

const inList = (param: Param, value: string) =>
	String(param ?? '')
		.split(',')
		.map((z) => z.trim())
		.includes(value)

export const CRITERIA: Record<CriterionId, CriterionDef> = {
	siren: {
		label: 'SIREN trouvé et confirmé par 2 indices',
		evaluate: (d) => (d.siren ? (d.indices >= 2 ? 'ok' : 'unknown') : 'ko'),
	},
	active: {
		label: 'Entreprise active (non radiée)',
		evaluate: (d) => (d.siren ? (d.active ? 'ok' : 'ko') : 'unknown'),
	},
	collective: {
		label: 'Aucune procédure collective (BODACC)',
		evaluate: (d) => (d.collective ? 'ko' : 'ok'),
	},
	effectif: {
		label: 'Effectif minimum',
		defaultParam: 50,
		unit: 'salariés',
		evaluate: (d, p) =>
			d.effectif === null ? 'unknown' : d.effectif >= Number(p) ? 'ok' : 'ko',
	},
	naf: {
		label: 'Code NAF dans la liste',
		defaultParam: '5229A, 6201Z, 6419Z',
		evaluate: (d, p) => (!d.naf ? 'unknown' : inList(p, d.naf) ? 'ok' : 'ko'),
	},
	pays: {
		label: 'Pays du siège',
		defaultParam: 'FR',
		evaluate: (d, p) => (!d.pays ? 'unknown' : inList(p, d.pays) ? 'ok' : 'ko'),
	},
	crm: {
		label: 'Absente du CRM',
		evaluate: (d) => (d.crm ? 'ko' : 'ok'),
	},
	rdap: {
		label: 'Titulaire RDAP non masqué',
		evaluate: (d) => (d.rdapMasque ? 'unknown' : 'ok'),
	},
	age: {
		label: 'Domaine créé depuis au moins',
		defaultParam: 30,
		unit: 'jours',
		evaluate: (d, p) => (d.age >= Number(p) ? 'ok' : 'ko'),
	},
}

export const CRITERION_IDS = Object.keys(CRITERIA) as CriterionId[]

export interface PipelineCriterion {
	id: CriterionId
	param: Param
	onFail: OnFail
	enabled: boolean
}

export function makeCriterion(
	id: CriterionId,
	param: Param,
	onFail: OnFail,
): PipelineCriterion {
	return {
		id,
		param: param === undefined ? CRITERIA[id].defaultParam : param,
		onFail,
		enabled: true,
	}
}

export const PRESETS: Record<string, ReadonlyArray<PipelineCriterion>> = {
	'Prospection B2B': [
		makeCriterion('siren', undefined, 'review'),
		makeCriterion('active', undefined, 'reject'),
		makeCriterion('collective', undefined, 'reject'),
		makeCriterion('effectif', 50, 'review'),
		makeCriterion('pays', 'FR', 'reject'),
		makeCriterion('crm', undefined, 'reject'),
	],
	'Veille sécurité': [
		makeCriterion('siren', undefined, 'review'),
		makeCriterion('rdap', undefined, 'review'),
		makeCriterion('age', 30, 'review'),
	],
	Minimal: [
		makeCriterion('siren', undefined, 'review'),
		makeCriterion('active', undefined, 'reject'),
	],
}

export interface DecisionRow {
	criterion: PipelineCriterion
	result: CriterionResult
}

export interface Decision {
	rows: DecisionRow[]
	verdict: Verdict
}

export function decide(
	pipeline: ReadonlyArray<PipelineCriterion>,
	dossier: Dossier,
): Decision {
	const rows = pipeline
		.filter((c) => c.enabled)
		.map((criterion) => ({
			criterion,
			result: CRITERIA[criterion.id].evaluate(dossier, criterion.param),
		}))
	let verdict: Verdict = 'qualified'
	for (const { criterion, result } of rows) {
		if (criterion.onFail === 'ignore') continue
		if (result === 'ko' && criterion.onFail === 'reject') verdict = 'rejected'
		else if (result !== 'ok' && verdict !== 'rejected') verdict = 'review'
	}
	return { rows, verdict }
}
