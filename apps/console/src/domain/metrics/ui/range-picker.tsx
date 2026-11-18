import { CalendarDays, ChevronLeft, ChevronRight } from 'lucide-react'
import { Popover as PopoverPrimitive } from 'radix-ui'
import { useState } from 'react'
import {
	type DraftRange,
	formatRange,
	isComplete,
	monthWeeks,
	pickDay,
	previewRange,
	shiftMonth,
} from '#/domain/metrics/calendar'
import { PERIODS, type Period } from '#/domain/metrics/model'
import { cn } from '#/lib/utils'

interface Props {
	period: Period
	range: DraftRange
	/** Dernier jour choisissable ; les jours suivants sont grisés. */
	today: string
	onChange: (period: Period, range: DraftRange) => void
}

const WEEKDAYS = [
	{ id: 'lun', label: 'L' },
	{ id: 'mar', label: 'M' },
	{ id: 'mer', label: 'M' },
	{ id: 'jeu', label: 'J' },
	{ id: 'ven', label: 'V' },
	{ id: 'sam', label: 'S' },
	{ id: 'dim', label: 'D' },
]
const MONTH_TITLE = new Intl.DateTimeFormat('fr-FR', {
	month: 'long',
	year: 'numeric',
	timeZone: 'UTC',
})
const FULL_DAY = new Intl.DateTimeFormat('fr-FR', {
	weekday: 'long',
	day: 'numeric',
	month: 'long',
	year: 'numeric',
	timeZone: 'UTC',
})
const PRESETS = PERIODS.filter((item) => item.value !== 'libre')

const asDate = (day: string) => new Date(`${day}T00:00:00Z`)

function Month({
	month,
	shown,
	today,
	onPick,
	onHover,
}: {
	month: string
	shown: DraftRange
	today: string
	onPick: (day: string) => void
	onHover: (day: string | null) => void
}) {
	return (
		<div className="w-[212px]">
			<div className="mb-1 text-center text-xs font-semibold capitalize">
				{MONTH_TITLE.format(asDate(month))}
			</div>
			<div className="grid grid-cols-7 text-center text-[10.5px] text-faint">
				{WEEKDAYS.map((weekday) => (
					<span key={weekday.id} className="py-1">
						{weekday.label}
					</span>
				))}
			</div>
			{monthWeeks(month).map((week) => (
				<div key={week[0]?.day} className="grid grid-cols-7">
					{week.map((cell) => {
						if (!cell.inMonth) return <span key={cell.day} />
						const start = cell.day === shown.from
						const end = cell.day === shown.to
						const inside =
							shown.from !== '' &&
							shown.to !== '' &&
							cell.day > shown.from &&
							cell.day < shown.to
						return (
							<button
								key={cell.day}
								type="button"
								disabled={cell.day > today}
								aria-label={FULL_DAY.format(asDate(cell.day))}
								aria-pressed={start || end}
								onClick={() => onPick(cell.day)}
								onMouseEnter={() => onHover(cell.day)}
								onMouseLeave={() => onHover(null)}
								onFocus={() => onHover(cell.day)}
								className={cn(
									'figures h-7 text-[12px] focus-visible:outline-2 focus-visible:outline-ring',
									'enabled:hover:bg-secondary disabled:text-faint/50',
									inside && 'bg-primary/12',
									(start || end) &&
										'rounded-[5px] bg-primary text-primary-foreground enabled:hover:bg-primary',
									cell.day === today && !start && !end && 'font-semibold',
								)}
							>
								{Number(cell.day.slice(8))}
							</button>
						)
					})}
				</div>
			))}
		</div>
	)
}

export function RangePicker({ period, range, today, onChange }: Props) {
	const [open, setOpen] = useState(false)
	const [draft, setDraft] = useState<DraftRange>({ from: '', to: '' })
	const [hover, setHover] = useState<string | null>(null)
	const [left, setLeft] = useState(shiftMonth(today, -1))
	const shown = previewRange(draft, hover)
	const label =
		period === 'libre'
			? formatRange(range)
			: (PERIODS.find((item) => item.value === period)?.label ?? '')

	const changeOpen = (next: boolean) => {
		setOpen(next)
		if (next) {
			setDraft(period === 'libre' ? range : { from: '', to: '' })
			setHover(null)
		}
	}
	const pick = (day: string) => {
		const next = pickDay(draft, day)
		setDraft(next)
		if (isComplete(next)) {
			onChange('libre', next)
			setOpen(false)
		}
	}
	const choose = (value: Period) => {
		onChange(value, range)
		setOpen(false)
	}

	return (
		<PopoverPrimitive.Root open={open} onOpenChange={changeOpen}>
			<PopoverPrimitive.Trigger asChild>
				<button
					type="button"
					aria-label={`Période : ${label}`}
					className="inline-flex h-7 items-center gap-1.5 rounded-md border border-input bg-card px-2 text-[13px] hover:bg-secondary focus-visible:outline-2 focus-visible:outline-ring"
				>
					<CalendarDays aria-hidden="true" className="size-3.5 text-faint" />
					{label}
				</button>
			</PopoverPrimitive.Trigger>
			<PopoverPrimitive.Portal>
				<PopoverPrimitive.Content
					align="start"
					sideOffset={6}
					className="z-50 flex max-w-[calc(100vw-16px)] flex-wrap gap-3 rounded-lg border bg-popover p-3 text-popover-foreground shadow-md"
				>
					<ul className="m-0 flex list-none flex-col gap-0.5 p-0 text-[13px]">
						{PRESETS.map((item) => (
							<li key={item.value}>
								<button
									type="button"
									aria-pressed={period === item.value}
									onClick={() => choose(item.value)}
									className={cn(
										'w-full rounded-md px-2 py-1 text-left hover:bg-secondary focus-visible:outline-2 focus-visible:outline-ring',
										period === item.value && 'bg-secondary font-medium',
									)}
								>
									{item.label}
								</button>
							</li>
						))}
					</ul>
					<div>
						<div className="mb-1 flex items-center justify-between">
							<button
								type="button"
								aria-label="Mois précédent"
								onClick={() => setLeft(shiftMonth(left, -1))}
								className="grid size-6 place-items-center rounded-md hover:bg-secondary focus-visible:outline-2 focus-visible:outline-ring"
							>
								<ChevronLeft aria-hidden="true" className="size-4" />
							</button>
							<span className="text-[11.5px] text-muted-foreground">
								{draft.from && !draft.to
									? 'Choisis la date de fin'
									: 'Choisis la date de début'}
							</span>
							<button
								type="button"
								aria-label="Mois suivant"
								disabled={shiftMonth(left, 1) >= `${today.slice(0, 7)}-01`}
								onClick={() => setLeft(shiftMonth(left, 1))}
								className="grid size-6 place-items-center rounded-md hover:bg-secondary disabled:opacity-40 focus-visible:outline-2 focus-visible:outline-ring"
							>
								<ChevronRight aria-hidden="true" className="size-4" />
							</button>
						</div>
						<div className="flex flex-wrap gap-4">
							<Month
								month={left}
								shown={shown}
								today={today}
								onPick={pick}
								onHover={setHover}
							/>
							<Month
								month={shiftMonth(left, 1)}
								shown={shown}
								today={today}
								onPick={pick}
								onHover={setHover}
							/>
						</div>
					</div>
				</PopoverPrimitive.Content>
			</PopoverPrimitive.Portal>
		</PopoverPrimitive.Root>
	)
}
