import { DEMO_PROJECTS, DEMO_TASKS } from '#/domain/demo'
import { slugify } from '#/domain/routines/routine-draft'
import { parseSchedule, scheduleText } from '#/domain/routines/schedule'
import type {
	Routine,
	RoutineRun,
	RunOrigin,
	RunTone,
} from '#/domain/routines/types'
import { NO_PROJECT } from '#/domain/routines/types'

interface Row {
	name: string
	agent: string
	lastWhen: string
	tone: RunTone
	label: string
	enabled: boolean
	schedule: string
}

const ROWS: ReadonlyArray<Row> = [
	{
		name: 'Analyse board (hebdomadaire)',
		agent: 'Board Analyst',
		lastWhen: '12 nov., 06:30',
		tone: 'success',
		label: 'terminée',
		enabled: true,
		schedule: 'Lun. 06:30',
	},
	{
		name: 'Dépendances et pipelines verts',
		agent: 'Patch Management & CI/CD',
		lastWhen: '17 nov., 06:00',
		tone: 'neutral',
		label: 'fusionnée',
		enabled: true,
		schedule: 'Chaque jour 06:00',
	},
	{
		name: 'Marketing : déclencheurs de publication',
		agent: 'Constance (Marketing)',
		lastWhen: 'Jamais',
		tone: 'neutral',
		label: 'en pause',
		enabled: false,
		schedule: 'Jours ouvrés 08:00',
	},
	{
		name: 'Marketing : plan de la semaine',
		agent: 'Constance (Marketing)',
		lastWhen: '16 nov., 09:30',
		tone: 'success',
		label: 'terminée',
		enabled: true,
		schedule: 'Lun. 09:30',
	},
	{
		name: 'Newsletter produit hebdomadaire',
		agent: 'Constance (Marketing)',
		lastWhen: '17 nov., 09:30',
		tone: 'violet',
		label: 'tâche créée',
		enabled: true,
		schedule: 'Mar. 09:30',
	},
	{
		name: 'Notoriété : bilan et plan de la semaine',
		agent: 'Maëva (Notoriété)',
		lastWhen: '16 nov., 08:30',
		tone: 'success',
		label: 'terminée',
		enabled: true,
		schedule: 'Lun. 08:30',
	},
	{
		name: 'Optimisation SEO du site (48 h)',
		agent: 'SEO & Website Optimizer',
		lastWhen: '17 nov., 09:00',
		tone: 'neutral',
		label: 'fusionnée',
		enabled: true,
		schedule: 'Toutes les 48 h',
	},
	{
		name: 'Relance des décisions ouvertes (J+2)',
		agent: 'Chief of Staff',
		lastWhen: '30 sept., 06:00',
		tone: 'danger',
		label: 'échec',
		enabled: false,
		schedule: 'Jours ouvrés 06:00',
	},
	{
		name: 'Revue de conformité ISO/IEC 42001',
		agent: 'Compliance ISO/IEC 42001',
		lastWhen: '16 nov., 06:00',
		tone: 'success',
		label: 'terminée',
		enabled: true,
		schedule: 'Lun. 06:00',
	},
	{
		name: 'Revue des accès externes du Drive',
		agent: 'Security Officer (RSSI + Blue Team)',
		lastWhen: 'Jamais',
		tone: 'neutral',
		label: 'jamais lancée',
		enabled: true,
		schedule: 'Le 1er du mois',
	},
	{
		name: 'Revue hebdomadaire des agents',
		agent: 'Chief of Staff',
		lastWhen: '16 nov., 06:00',
		tone: 'success',
		label: 'terminée',
		enabled: true,
		schedule: 'Lun. 06:00',
	},
]

const taskId = (n: number): string =>
	DEMO_TASKS[n % DEMO_TASKS.length]?.id ?? 'NAT-1'

function history(row: Row, index: number): ReadonlyArray<RoutineRun> {
	if (row.lastWhen === 'Jamais') return []
	const entry = (
		n: number,
		when: string,
		origin: RunOrigin,
		tone: RunTone,
		label: string,
		duration: string,
	): RoutineRun => ({
		id: `seed-${index}-${n}`,
		when,
		origin,
		tone,
		label,
		duration,
		taskId: taskId(index * 2 + n),
	})
	return [
		entry(0, row.lastWhen, 'Planning', row.tone, row.label, '1 min 12'),
		entry(1, 'il y a 1 sem.', 'Planning', 'success', 'terminée', '48 s'),
		entry(2, 'il y a 2 sem.', 'Manuel', 'success', 'terminée', '3 min 05'),
		entry(3, 'il y a 3 sem.', 'Planning', 'danger', 'échec', '12 min'),
		entry(4, 'il y a 4 sem.', 'Planning', 'success', 'terminée', '2 min 30'),
	]
}

/** Copie neuve à chaque appel : le store peut la modifier sans toucher aux fixtures. */
export function buildRoutines(): ReadonlyArray<Routine> {
	return ROWS.map((row, index) => ({
		id: slugify(row.name),
		name: row.name,
		agent: row.agent,
		enabled: row.enabled,
		lastRun: { when: row.lastWhen, tone: row.tone, label: row.label },
		listSchedule:
			scheduleText(parseSchedule(row.schedule)) === row.schedule
				? undefined
				: row.schedule,
		instruction: `Exécuter « ${row.name} », documenter le résultat dans la tâche et escalader au Chief of Staff en cas de blocage.`,
		project:
			index % 2 === 1
				? (DEMO_PROJECTS[index % DEMO_PROJECTS.length]?.name ?? NO_PROJECT)
				: NO_PROJECT,
		priority: 'Moyenne',
		titleTemplate: '{nom} · {date}',
		ifOpen: 'Ignorer si déjà ouverte',
		ifMissed: 'Rattraper une seule fois',
		timezone: 'Europe/Paris',
		triggers: [
			{
				kind: 'planning',
				id: `planning-${index}`,
				enabled: row.enabled,
				schedule: parseSchedule(row.schedule),
			},
		],
		runs: history(row, index),
	}))
}
