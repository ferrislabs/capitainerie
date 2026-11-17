import type { Day, Schedule } from '#/domain/routines/schedule'
import { DAYS } from '#/domain/routines/schedule'
import type { RoutineSettings } from '#/domain/routines/types'
import { NO_PROJECT } from '#/domain/routines/types'

export const DEFAULT_SETTINGS: RoutineSettings = {
	name: '',
	instruction: '',
	agent: 'Chief of Staff',
	project: NO_PROJECT,
	priority: 'Moyenne',
	titleTemplate: '{nom} · {date}',
	ifOpen: 'Ignorer si déjà ouverte',
	ifMissed: 'Rattraper une seule fois',
	timezone: 'Europe/Paris',
}

const MAX_SLUG = 24

export const slugify = (text: string): string =>
	text
		.normalize('NFD')
		.replace(/[̀-ͯ]/g, '')
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '')

export function uniqueId(base: string, taken: ReadonlyArray<string>): string {
	const root = base || 'routine'
	if (!taken.includes(root)) return root
	let n = 2
	while (taken.includes(`${root}-${n}`)) n++
	return `${root}-${n}`
}

export const nameError = (name: string): string | null =>
	name.trim() ? null : 'Le nom est obligatoire'

export const scheduleError = (schedule: Schedule): string | null =>
	schedule.frequency === 'Toutes les semaines' && schedule.days.length === 0
		? 'Choisis au moins un jour'
		: null

export const toggleDay = (
	days: ReadonlyArray<Day>,
	day: Day,
): ReadonlyArray<Day> =>
	(days.includes(day) ? days.filter((d) => d !== day) : [...days, day]).sort(
		(a, b) => DAYS.indexOf(a) - DAYS.indexOf(b),
	)

export const webhookUrl = (routineName: string, suffix: string): string =>
	`https://paperclip.getnatalia.com/hooks/routines/${slugify(routineName).slice(0, MAX_SLUG)}-${suffix}`

export const maskedSecret = (secret: string): string =>
	`whsec_••••••••${secret}`

export const settingsOf = (routine: RoutineSettings): RoutineSettings => ({
	name: routine.name,
	instruction: routine.instruction,
	agent: routine.agent,
	project: routine.project,
	priority: routine.priority,
	titleTemplate: routine.titleTemplate,
	ifOpen: routine.ifOpen,
	ifMissed: routine.ifMissed,
	timezone: routine.timezone,
})
