import type { Schemas } from '#/api/api.client'

export type Frequency = Schemas.FrequencyDto

export const FREQUENCIES: ReadonlyArray<{ value: Frequency; label: string }> = [
	{ value: 'every_hours', label: 'Toutes les N heures' },
	{ value: 'daily', label: 'Tous les jours' },
	{ value: 'weekly', label: 'Certains jours de la semaine' },
	{ value: 'monthly', label: 'Le 1er de chaque mois' },
	{ value: 'cron', label: 'Expression cron' },
]

export const DAYS = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'] as const

export type Day = (typeof DAYS)[number]

export const DAY_LABELS: Record<Day, string> = {
	mon: 'Lun',
	tue: 'Mar',
	wed: 'Mer',
	thu: 'Jeu',
	fri: 'Ven',
	sat: 'Sam',
	sun: 'Dim',
}

export const MAX_EVERY_HOURS = 744
const DEFAULT_TIME = '08:00'
const TIME_PATTERN = /^([01]\d|2[0-3]):[0-5]\d$/
const CRON_FIELDS = 5

/** Planning en cours d'édition : tous les champs restent présents pour changer de fréquence sans rien perdre. */
export interface ScheduleDraft {
	frequency: Frequency
	hours: number
	time: string
	days: ReadonlyArray<Day>
	cron: string
}

export const defaultSchedule = (): ScheduleDraft => ({
	frequency: 'weekly',
	hours: 24,
	time: DEFAULT_TIME,
	days: ['mon'],
	cron: '',
})

const isDay = (value: string): value is Day =>
	(DAYS as ReadonlyArray<string>).includes(value)

export function scheduleDraftOf(
	dto: Schemas.ScheduleDto | null | undefined,
): ScheduleDraft {
	const base = defaultSchedule()
	if (!dto) return base
	const days = (dto.days ?? []).filter(isDay)
	return {
		frequency: dto.frequency,
		hours: dto.hours ?? base.hours,
		time: dto.time ?? base.time,
		days: days.length > 0 ? days : base.days,
		cron: dto.cron ?? '',
	}
}

export function scheduleDto(draft: ScheduleDraft): Schemas.ScheduleDto {
	switch (draft.frequency) {
		case 'every_hours':
			return { frequency: 'every_hours', hours: draft.hours }
		case 'daily':
		case 'monthly':
			return { frequency: draft.frequency, time: draft.time }
		case 'weekly':
			return { frequency: 'weekly', time: draft.time, days: [...draft.days] }
		case 'cron':
			return { frequency: 'cron', cron: draft.cron.trim() }
	}
}

/** Message d'erreur du planning, ou `null` s'il est valide. */
export function scheduleError(draft: ScheduleDraft): string | null {
	switch (draft.frequency) {
		case 'every_hours':
			return Number.isInteger(draft.hours) &&
				draft.hours >= 1 &&
				draft.hours <= MAX_EVERY_HOURS
				? null
				: `Indique un nombre d'heures entre 1 et ${MAX_EVERY_HOURS}.`
		case 'cron':
			return draft.cron.trim().split(/\s+/).filter(Boolean).length ===
				CRON_FIELDS
				? null
				: "L'expression cron doit avoir 5 champs : minute heure jour mois jour-semaine."
		case 'weekly':
			if (draft.days.length === 0) return 'Choisis au moins un jour.'
			break
		default:
			break
	}
	return TIME_PATTERN.test(draft.time)
		? null
		: "L'heure doit être au format HH:MM (ex. 08:30)."
}

export const toggleDay = (
	days: ReadonlyArray<Day>,
	day: Day,
): ReadonlyArray<Day> =>
	(days.includes(day) ? days.filter((d) => d !== day) : [...days, day]).sort(
		(a, b) => DAYS.indexOf(a) - DAYS.indexOf(b),
	)

const WORKING_DAYS: ReadonlyArray<Day> = ['mon', 'tue', 'wed', 'thu', 'fri']

function daysText(days: ReadonlyArray<string>): string {
	const known = DAYS.filter((d) => days.includes(d))
	if (known.length === DAYS.length) return 'Chaque jour'
	if (
		known.length === WORKING_DAYS.length &&
		WORKING_DAYS.every((d) => known.includes(d))
	)
		return 'Jours ouvrés'
	return known.map((d) => DAY_LABELS[d]).join(', ') || 'Aucun jour'
}

/** Résumé d'un planning tel que l'API le renvoie, ex. « Lun, Mer 08:00 ». */
export function scheduleText(dto: Schemas.ScheduleDto): string {
	const time = dto.time ?? DEFAULT_TIME
	switch (dto.frequency) {
		case 'every_hours':
			return dto.hours === 1
				? 'Toutes les heures'
				: `Toutes les ${dto.hours ?? 1} h`
		case 'daily':
			return `Chaque jour ${time}`
		case 'weekly':
			return `${daysText(dto.days ?? [])} ${time}`
		case 'monthly':
			return `Le 1er du mois ${time}`
		case 'cron':
			return `Cron ${dto.cron ?? ''}`.trim()
	}
}
