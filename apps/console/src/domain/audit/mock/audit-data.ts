export type ActivityActor = 'agent' | 'human' | 'system'

export interface ActivityEvent {
	who: string
	what: string
	when: string
	actor: ActivityActor
}

export const ACTIVITY: ReadonlyArray<ActivityEvent> = [
	{
		who: 'Iris (Relais retours)',
		what: "a libéré le bail d'environnement",
		when: 'il y a 2 min',
		actor: 'agent',
	},
	{
		who: 'Système',
		what: "a enregistré le coût d'une exécution",
		when: 'il y a 2 min',
		actor: 'system',
	},
	{
		who: 'Iris (Relais retours)',
		what: "a pris un bail d'environnement",
		when: 'il y a 4 min',
		actor: 'agent',
	},
	{
		who: 'François-Guillaume Ribreau',
		what: 'a testé le MCP drive-steward : journal, sur « Organisation du Drive »',
		when: 'il y a 7 min',
		actor: 'human',
	},
	{
		who: 'François-Guillaume Ribreau',
		what: 'a lancé un test du MCP drive-steward : inventaire, sur « Organisation du Drive »',
		when: 'il y a 7 min',
		actor: 'human',
	},
	{
		who: 'Baptiste Parmantier',
		what: 'a lu NAT-679 « Diff gate SEO : la sonde de mise en page plante »',
		when: 'il y a 30 min',
		actor: 'human',
	},
	{
		who: 'Baptiste Parmantier',
		what: 'a lu NAT-579 « [archive] Point Copil IA : infrastructure, formation »',
		when: 'il y a 30 min',
		actor: 'human',
	},
]

export type AuditRunStatus = 'running' | 'success' | 'danger'

export interface AuditRun {
	agent: string
	trigger: string
	status: AuditRunStatus
	statusLabel: string
	duration: string
	when: string
}

export const AUDIT_RUNS: ReadonlyArray<AuditRun> = [
	{
		agent: 'Security Officer (RSSI + Blue Team)',
		trigger: 'Revue sécurité quotidienne',
		status: 'running',
		statusLabel: 'en cours',
		duration: '85 h 32',
		when: 'en file depuis 4 j',
	},
	{
		agent: 'Iris (Relais retours)',
		trigger: 'Heartbeat planifié',
		status: 'success',
		statusLabel: 'terminée',
		duration: '14 s',
		when: 'il y a 2 min',
	},
	{
		agent: 'Hippolyte (Relais archivage)',
		trigger: 'Heartbeat planifié',
		status: 'success',
		statusLabel: 'terminée',
		duration: '9 s',
		when: 'il y a 31 min',
	},
	{
		agent: 'Iris (Relais retours)',
		trigger: 'Heartbeat planifié',
		status: 'success',
		statusLabel: 'terminée',
		duration: '12 s',
		when: 'il y a 35 min',
	},
	{
		agent: 'Archibald (Archivage comptable)',
		trigger: 'NAT-684 Pièce reçue sur Chatwoot',
		status: 'success',
		statusLabel: 'terminée',
		duration: '1 min 42',
		when: 'il y a 2 h',
	},
	{
		agent: 'Dev (cycle autonome)',
		trigger: 'NAT-680 Sonde de mise en page',
		status: 'danger',
		statusLabel: 'échec',
		duration: '3 min 05',
		when: 'il y a 6 h',
	},
]

export interface DailyRuns {
	day: string
	succeeded: number
	failed: number
}

const SUCCEEDED = [18, 15, 20, 17, 9, 19, 24, 17, 20, 22, 17, 24, 36, 23]
const FAILED = [1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 2, 1, 0, 1]

export const DAILY_RUNS: ReadonlyArray<DailyRuns> = SUCCEEDED.map(
	(succeeded, index) => ({
		day: `${4 + index} nov.`,
		succeeded,
		failed: FAILED[index] ?? 0,
	}),
)

export interface CostTile {
	label: string
	value: string
	hint: string
}

export const COST_TILES: ReadonlyArray<CostTile> = [
	{ label: 'Dépense API', value: '0,00 $', hint: '0 entrée · 0 sortie' },
	{
		label: 'Jetons',
		value: '404,9 M',
		hint: 'en entrée · 1,9 M en sortie',
	},
	{ label: 'Exécutions abonnement', value: '55', hint: 'sur 14 jours' },
	{ label: 'Budget utilisé', value: '0 %', hint: '0,00 $ sur 400 $ ce mois' },
]

export interface AgentCost {
	agent: string
	input: string
	cached: string
	output: string
	runs: number
	cost: string
}

export const AGENT_COSTS: ReadonlyArray<AgentCost> = [
	{
		agent: 'Patch Management & CI/CD',
		input: '7,8 M',
		cached: '7,1 M',
		output: '82,3 k',
		runs: 18,
		cost: '0,00 $',
	},
	{
		agent: 'Archibald (Archivage comptable)',
		input: '5,0 M',
		cached: '4,1 M',
		output: '26,0 k',
		runs: 31,
		cost: '0,00 $',
	},
	{
		agent: 'Comptes rendus daily',
		input: '1,5 M',
		cached: '1,4 M',
		output: '13,5 k',
		runs: 2,
		cost: '0,00 $',
	},
	{
		agent: 'Organisation du Drive',
		input: '211,6 k',
		cached: '155,6 k',
		output: '2,1 k',
		runs: 4,
		cost: '0,00 $',
	},
	{
		agent: 'Iris (Relais retours)',
		input: '0',
		cached: '0',
		output: '0',
		runs: 0,
		cost: '0,00 $',
	},
]

export interface Budget {
	scope: string
	kind: 'organisation' | 'agent'
	spent: number
	limit: number
}

export const BUDGETS: ReadonlyArray<Budget> = [
	{ scope: 'Organisation', kind: 'organisation', spent: 0, limit: 400 },
	{ scope: 'Dev (cycle autonome)', kind: 'agent', spent: 0, limit: 120 },
	{ scope: 'Constance (Marketing)', kind: 'agent', spent: 0, limit: 60 },
	{
		scope: 'Security Officer (RSSI + Blue Team)',
		kind: 'agent',
		spent: 0,
		limit: 80,
	},
]

export type Severity = 'critical' | 'warning' | 'info' | 'success' | 'created'

export interface TimelineEvent {
	time: string
	text: string
	severity: Severity
}

export const TIMELINE: ReadonlyArray<TimelineEvent> = [
	{
		time: '07:00',
		text: 'Security Officer : limite de terminal atteinte sur NAT-611',
		severity: 'critical',
	},
	{
		time: '09:30',
		text: 'Newsletter produit hebdomadaire : tâche créée',
		severity: 'created',
	},
	{
		time: '11:12',
		text: 'Archibald : NAT-684 archivée',
		severity: 'success',
	},
	{
		time: '14:05',
		text: 'Chief of Staff : NAT-679 bloquée par NAT-680',
		severity: 'warning',
	},
	{
		time: '14:20',
		text: 'François-Guillaume Ribreau : test du MCP drive-steward',
		severity: 'info',
	},
]
