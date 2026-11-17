export interface LiveRun {
	agentId: string
	label: string
	taskId: string | null
	scheduled: boolean
	footer: string
}

export const LIVE_RUNS: ReadonlyArray<LiveRun> = [
	{
		agentId: 'security-officer',
		label: 'Revue sécurité quotidienne',
		taskId: 'NAT-611',
		scheduled: false,
		footer: 'En file depuis 4 j',
	},
	{
		agentId: 'iris',
		label: 'Heartbeat planifié',
		taskId: null,
		scheduled: true,
		footer: 'Terminé il y a 2 min',
	},
	{
		agentId: 'hippolyte',
		label: 'Heartbeat planifié',
		taskId: null,
		scheduled: true,
		footer: 'Terminé il y a 31 min',
	},
	{
		agentId: 'iris',
		label: 'Heartbeat planifié',
		taskId: null,
		scheduled: true,
		footer: 'Terminé il y a 35 min',
	},
]

export const OTHER_RUNS_LABEL = '46 autres exécutions actives ou récentes'

export type AttentionTarget = 'routines' | 'task' | 'tasks' | 'approvals'
export type Severity = 'crit' | 'warn' | 'info'

export interface AttentionItem {
	severity: Severity
	title: string
	detail: string
	action: string
	target: AttentionTarget
}

export const ATTENTION: ReadonlyArray<AttentionItem> = [
	{
		severity: 'crit',
		title: 'Échec de routine : « Relance des décisions ouvertes (J+2) »',
		detail: 'Dernier passage le 30 sept. Routine en pause, 1 échec.',
		action: 'Ouvrir la routine',
		target: 'routines',
	},
	{
		severity: 'crit',
		title: "NAT-611 : l'agent a signalé une limite de terminal",
		detail: 'Reprise automatique dans 2 h 27, tentative 1. Bloqué depuis 85 h.',
		action: 'Ouvrir la tâche',
		target: 'task',
	},
	{
		severity: 'warn',
		title: '65 tâches bloquées sur 122 ouvertes',
		detail:
			'NAT-679 attend NAT-680 ; NAT-676 attend NAT-679 ; NAT-674 couvert par 2 dépendances.',
		action: 'Voir les bloquées',
		target: 'tasks',
	},
	{
		severity: 'warn',
		title: '2 routines en pause',
		detail:
			'Déclencheurs de publication (jamais lancée) et relance des décisions ouvertes.',
		action: 'Voir les routines',
		target: 'routines',
	},
	{
		severity: 'info',
		title: 'Aucune approbation en attente',
		detail: "Le board n'a rien à valider.",
		action: 'Approbations',
		target: 'approvals',
	},
]

export interface RecentActivity {
	who: string
	what: string
	when: string
	kind: 'agent' | 'human'
	initials: string
}

export const RECENT_ACTIVITY: ReadonlyArray<RecentActivity> = [
	{
		who: 'Iris',
		what: "a libéré le bail d'environnement",
		when: '2 min',
		kind: 'agent',
		initials: '',
	},
	{
		who: 'Système',
		what: "a enregistré le coût d'une exécution",
		when: '2 min',
		kind: 'human',
		initials: 'SY',
	},
	{
		who: 'Iris',
		what: "a pris un bail d'environnement",
		when: '4 min',
		kind: 'agent',
		initials: '',
	},
	{
		who: 'François-Guillaume Ribreau',
		what: 'a testé le MCP drive-steward : journal, sur « Organisation du Drive »',
		when: '7 min',
		kind: 'human',
		initials: 'FG',
	},
	{
		who: 'François-Guillaume Ribreau',
		what: 'a lancé un test du MCP drive-steward : inventaire, sur « Organisation du Drive »',
		when: '7 min',
		kind: 'human',
		initials: 'FG',
	},
	{
		who: 'Baptiste Parmantier',
		what: 'a lu NAT-679 « Diff gate SEO : la sonde de mise en page plante »',
		when: '30 min',
		kind: 'human',
		initials: 'BA',
	},
]

export const RUNS_14: ReadonlyArray<ReadonlyArray<number>> = [
	[18, 1],
	[15, 1],
	[20, 1],
	[17, 1],
	[9, 0],
	[19, 1],
	[24, 1],
	[17, 0],
	[20, 1],
	[22, 1],
	[17, 2],
	[24, 1],
	[36, 0],
	[23, 1],
]

const TASKS_14_RAW: ReadonlyArray<readonly number[]> = [
	[3, 0, 0, 5, 0],
	[1, 0, 0, 4, 0],
	[2, 1, 0, 6, 1],
	[2, 1, 0, 6, 1],
	[0, 0, 0, 3, 0],
	[3, 2, 0, 7, 3],
	[4, 2, 1, 9, 5],
	[1, 0, 0, 6, 0],
	[2, 1, 0, 8, 0],
	[3, 1, 0, 9, 2],
	[1, 1, 0, 8, 1],
	[2, 1, 0, 10, 2],
	[6, 2, 1, 24, 9],
	[3, 1, 0, 11, 2],
]

export const TASKS_14: ReadonlyArray<ReadonlyArray<number>> = TASKS_14_RAW.map(
	(r) => [r[3] ?? 0, r[1] ?? 0, r[2] ?? 0, r[0] ?? 0, r[4] ?? 0],
)

export const SUCCESS_14: ReadonlyArray<number> = [
	97, 96, 98, 96, 97, 96, 97, 98, 98, 96, 95, 97, 98, 97,
]

export const AXIS_DATES: ReadonlyArray<string> = [
	'4 nov.',
	'10 nov.',
	'17 nov.',
]
