export const MIN_TOTAL_MS = 1000

const SECOND = 1000
const MINUTE = 60 * SECOND
const HOUR = 60 * MINUTE

const TICK_STEPS_MS = [
	SECOND,
	2 * SECOND,
	5 * SECOND,
	10 * SECOND,
	15 * SECOND,
	30 * SECOND,
	MINUTE,
	2 * MINUTE,
	5 * MINUTE,
	10 * MINUTE,
	15 * MINUTE,
	30 * MINUTE,
	HOUR,
	2 * HOUR,
	3 * HOUR,
	6 * HOUR,
	12 * HOUR,
	24 * HOUR,
]

const MAX_TICKS = 8

export interface AxisRun {
	created_at: string
	started_at?: string | null | undefined
	finished_at?: string | null | undefined
}

export interface AxisStep {
	started_at: string
	finished_at?: string | null | undefined
	status: 'running' | 'succeeded' | 'failed' | 'skipped'
}

export interface Axis {
	startMs: number
	totalMs: number
}

export interface Tick {
	ms: number
	leftPct: number
	label: string
}

export interface BlockPosition {
	leftPct: number
	widthPct: number
}

function parse(iso: string | null | undefined): number | undefined {
	if (!iso) return undefined
	const ms = Date.parse(iso)
	return Number.isNaN(ms) ? undefined : ms
}

function stepStartMs(step: AxisStep): number | undefined {
	return parse(step.started_at)
}

function stepEndMs(step: AxisStep, nowMs: number): number | undefined {
	const finished = parse(step.finished_at)
	if (finished !== undefined) return finished
	return step.status === 'running' ? nowMs : stepStartMs(step)
}

export function formatTick(ms: number): string {
	const totalSeconds = Math.round(ms / SECOND)
	if (totalSeconds < 60) return `${totalSeconds} s`
	if (totalSeconds < 3600) {
		const minutes = Math.floor(totalSeconds / 60)
		const seconds = totalSeconds % 60
		return seconds === 0 ? `${minutes} min` : `${minutes} min ${seconds} s`
	}
	const hours = Math.floor(totalSeconds / 3600)
	const minutes = Math.floor((totalSeconds % 3600) / 60)
	return minutes === 0 ? `${hours} h` : `${hours} h ${minutes} min`
}

export function tickStepMs(totalMs: number): number {
	const found = TICK_STEPS_MS.find((step) => totalMs / step <= MAX_TICKS)
	return found ?? TICK_STEPS_MS[TICK_STEPS_MS.length - 1]
}

export function buildTicks(totalMs: number): Tick[] {
	const span = Math.max(totalMs, MIN_TOTAL_MS)
	const step = tickStepMs(span)
	const ticks: Tick[] = []
	for (let ms = 0; ms <= span; ms += step) {
		ticks.push({ ms, leftPct: (ms / span) * 100, label: formatTick(ms) })
	}
	return ticks
}

export function computeAxis(
	run: AxisRun,
	steps: ReadonlyArray<AxisStep>,
	nowMs: number,
	active: boolean,
): Axis {
	const runStart = parse(run.started_at) ?? parse(run.created_at) ?? nowMs
	const stepStarts = steps.flatMap((step) => stepStartMs(step) ?? [])
	const startMs = Math.min(runStart, ...stepStarts)

	const runEnd = parse(run.finished_at) ?? (active ? nowMs : runStart)
	const stepEnds = steps.flatMap((step) => stepEndMs(step, nowMs) ?? [])
	const endMs = Math.max(runEnd, ...stepEnds)

	return { startMs, totalMs: Math.max(endMs - startMs, MIN_TOTAL_MS) }
}

export function blockPosition(
	step: AxisStep,
	axis: Axis,
	nowMs: number,
): BlockPosition {
	const start = stepStartMs(step) ?? axis.startMs
	const end = stepEndMs(step, nowMs) ?? start
	const leftPct = clampPct(((start - axis.startMs) / axis.totalMs) * 100)
	const rawWidth = ((Math.max(end, start) - start) / axis.totalMs) * 100
	return { leftPct, widthPct: Math.min(Math.max(rawWidth, 0), 100 - leftPct) }
}

function clampPct(value: number): number {
	return Math.min(Math.max(value, 0), 100)
}
