import { useId } from 'react'
import { toggleDay } from '#/domain/routines/routine-draft'
import {
	DAYS,
	FREQUENCIES,
	type Frequency,
	type Schedule,
} from '#/domain/routines/schedule'
import { SelectField } from '#/domain/routines/ui/select-field'
import { CTL, Fr } from '#/domain/sources/ui/mockup'
import { cn } from '#/lib/utils'

interface Props {
	schedule: Schedule
	onChange: (patch: Partial<Schedule>) => void
}

const isFrequency = (value: string): value is Frequency =>
	FREQUENCIES.some((f) => f === value)

const label = (day: string) => `${day.charAt(0).toUpperCase()}${day.slice(1)}`

export function ScheduleFields({ schedule, onChange }: Props) {
	const prefix = useId()
	const { frequency } = schedule
	return (
		<>
			<SelectField
				label="Fréquence"
				value={frequency}
				options={FREQUENCIES}
				onChange={(value) => {
					if (isFrequency(value)) onChange({ frequency: value })
				}}
			/>
			{frequency === 'Toutes les N heures' ? (
				<Fr label={<label htmlFor={`${prefix}-hours`}>Toutes les</label>}>
					<input
						id={`${prefix}-hours`}
						className={cn(CTL, 'w-[90px]')}
						type="number"
						min={1}
						max={168}
						value={schedule.hours}
						onChange={(event) =>
							onChange({ hours: Math.max(1, Number(event.target.value) || 1) })
						}
					/>{' '}
					<span className="text-muted-foreground">heures</span>
				</Fr>
			) : null}
			{frequency === 'Toutes les semaines' ? (
				<Fr label="Jours">
					<fieldset className="m-0 flex min-w-0 flex-wrap gap-1.5 border-0 p-0">
						<legend className="sr-only">Jours</legend>
						{DAYS.map((day) => (
							<label
								key={day}
								htmlFor={`${prefix}-${day}`}
								className="inline-flex cursor-pointer items-center gap-1.5 rounded-full border bg-background px-[9px] py-[3px] text-[12px] text-foreground"
							>
								<input
									id={`${prefix}-${day}`}
									type="checkbox"
									className="mx-0 my-[3px] ml-1 mr-[3px]"
									checked={schedule.days.includes(day)}
									onChange={() =>
										onChange({ days: toggleDay(schedule.days, day) })
									}
								/>
								{label(day)}
							</label>
						))}
					</fieldset>
				</Fr>
			) : null}
			{frequency !== 'Toutes les N heures' &&
			frequency !== 'Expression cron' ? (
				<Fr label={<label htmlFor={`${prefix}-time`}>Heure</label>}>
					<input
						id={`${prefix}-time`}
						className={cn(CTL, 'w-[90px] font-mono text-[11.5px]')}
						placeholder="08:00"
						value={schedule.time}
						onChange={(event) => onChange({ time: event.target.value })}
					/>
				</Fr>
			) : null}
			{frequency === 'Expression cron' ? (
				<Fr
					label={<label htmlFor={`${prefix}-cron`}>Expression cron</label>}
					hint="minute heure jour mois jour-semaine"
				>
					<input
						id={`${prefix}-cron`}
						className={cn(CTL, 'font-mono text-[11.5px]')}
						placeholder="30 6 * * 1"
						value={schedule.cron}
						onChange={(event) => onChange({ cron: event.target.value })}
					/>
				</Fr>
			) : null}
		</>
	)
}
