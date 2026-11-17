export const FREQUENCIES = [
	'Toutes les N heures',
	'Tous les jours',
	'Toutes les semaines',
	'Tous les mois',
	'Expression cron',
] as const

export type Frequency = (typeof FREQUENCIES)[number]

export const DAYS = ['lun', 'mar', 'mer', 'jeu', 'ven', 'sam', 'dim'] as const

export type Day = (typeof DAYS)[number]

export interface Schedule {
	frequency: Frequency
	hours: number
	time: string
	days: ReadonlyArray<Day>
	cron: string
}

const DEFAULT_TIME = '08:00'
const SEARCH_DAYS = 60
const SEARCH_MONTHS = 24

export const defaultSchedule = (): Schedule => ({
	frequency: 'Toutes les semaines',
	hours: 24,
	time: DEFAULT_TIME,
	days: ['lun'],
	cron: '',
})

const capitalize = (word: string) =>
	word.charAt(0).toUpperCase() + word.slice(1)

export function parseSchedule(text: string): Schedule {
	const schedule: Schedule = {
		...defaultSchedule(),
		time: text.match(/(\d{2}:\d{2})/)?.[1] ?? DEFAULT_TIME,
	}
	if (/Chaque jour/.test(text))
		return { ...schedule, frequency: 'Tous les jours' }
	if (/Jours ouvrés/.test(text)) return { ...schedule, days: DAYS.slice(0, 5) }
	const hours = text.match(/Toutes les (\d+) h/)?.[1]
	if (hours)
		return {
			...schedule,
			frequency: 'Toutes les N heures',
			hours: Number(hours),
		}
	if (/1er du mois/.test(text))
		return { ...schedule, frequency: 'Tous les mois' }
	const day = DAYS.find((d) => new RegExp(`^${d}`, 'i').test(text))
	return day ? { ...schedule, days: [day] } : schedule
}

const isWorkingDays = (days: ReadonlyArray<Day>) =>
	days.length === 5 && !days.includes('sam') && !days.includes('dim')

export function scheduleText(schedule: Schedule): string {
	const time = schedule.time || DEFAULT_TIME
	switch (schedule.frequency) {
		case 'Toutes les N heures':
			return `Toutes les ${schedule.hours || 1} h`
		case 'Tous les jours':
			return `Chaque jour ${time}`
		case 'Toutes les semaines': {
			const days = isWorkingDays(schedule.days)
				? 'Jours ouvrés'
				: schedule.days.map(capitalize).join(', ') || 'Aucun jour'
			return `${days} ${time}`
		}
		case 'Tous les mois':
			return `Le 1er du mois ${time}`
		case 'Expression cron':
			return schedule.cron || '* * * * *'
	}
}

const weekdayOf = (date: Date): Day => DAYS[(date.getDay() + 6) % 7] ?? 'lun'

/** Prochains passages à partir de `now` ; une expression cron est calculée côté serveur. */
export function nextRuns(
	schedule: Schedule,
	now: Date,
	count: number,
): ReadonlyArray<Date> {
	const [hour = 0, minute = 0] = (schedule.time || DEFAULT_TIME)
		.split(':')
		.map((part) => Number(part) || 0)
	const out: Date[] = []
	if (schedule.frequency === 'Toutes les N heures') {
		const cursor = new Date(now)
		cursor.setMinutes(0, 0, 0)
		cursor.setHours(cursor.getHours() + 1)
		for (let k = 0; k < count; k++) {
			out.push(new Date(cursor))
			cursor.setHours(cursor.getHours() + (schedule.hours || 1))
		}
	} else if (
		schedule.frequency === 'Tous les jours' ||
		schedule.frequency === 'Toutes les semaines'
	) {
		for (let k = 0; out.length < count && k < SEARCH_DAYS; k++) {
			const day = new Date(now)
			day.setDate(day.getDate() + k)
			day.setHours(hour, minute, 0, 0)
			if (day <= now) continue
			if (
				schedule.frequency === 'Tous les jours' ||
				schedule.days.includes(weekdayOf(day))
			)
				out.push(day)
		}
	} else if (schedule.frequency === 'Tous les mois') {
		for (let k = 0; out.length < count && k < SEARCH_MONTHS; k++) {
			const day = new Date(
				now.getFullYear(),
				now.getMonth() + k,
				1,
				hour,
				minute,
			)
			if (day > now) out.push(day)
		}
	}
	return out
}

export const formatRun = (date: Date): string =>
	date.toLocaleString('fr-FR', {
		weekday: 'short',
		day: 'numeric',
		month: 'short',
		hour: '2-digit',
		minute: '2-digit',
	})
