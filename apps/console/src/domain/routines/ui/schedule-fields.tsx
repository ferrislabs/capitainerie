import { useId } from 'react'
import {
	DAY_LABELS,
	DAYS,
	FREQUENCIES,
	MAX_EVERY_HOURS,
	type ScheduleDraft,
	toggleDay,
} from '#/domain/routines/schedule'
import { SelectField } from '#/domain/routines/ui/select-field'
import { CTL, Fr } from '#/domain/sources/ui/mockup'
import { cn } from '#/lib/utils'

interface Props {
	schedule: ScheduleDraft
	onChange: (patch: Partial<ScheduleDraft>) => void
}

export function ScheduleFields({ schedule, onChange }: Props) {
	const prefix = useId()
	const { frequency } = schedule
	return (
		<>
			<SelectField
				label="Fréquence"
				value={frequency}
				options={FREQUENCIES}
				onChange={(value) => onChange({ frequency: value })}
			/>
			{frequency === 'every_hours' ? (
				<Fr
					label={<label htmlFor={`${prefix}-hours`}>Toutes les</label>}
					hint="Le premier passage tombe à l'heure pile suivante."
				>
					<input
						id={`${prefix}-hours`}
						className={cn(CTL, 'w-[90px]')}
						type="number"
						min={1}
						max={MAX_EVERY_HOURS}
						value={schedule.hours}
						onChange={(event) =>
							onChange({ hours: Math.trunc(Number(event.target.value)) || 0 })
						}
					/>{' '}
					<span className="text-muted-foreground">heures</span>
				</Fr>
			) : null}
			{frequency === 'weekly' ? (
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
									className="mx-0 my-[3px] mr-[3px] ml-1"
									checked={schedule.days.includes(day)}
									onChange={() =>
										onChange({ days: toggleDay(schedule.days, day) })
									}
								/>
								{DAY_LABELS[day]}
							</label>
						))}
					</fieldset>
				</Fr>
			) : null}
			{frequency === 'daily' ||
			frequency === 'weekly' ||
			frequency === 'monthly' ? (
				<Fr
					label={<label htmlFor={`${prefix}-time`}>Heure</label>}
					hint="Dans le fuseau de la routine."
				>
					<input
						id={`${prefix}-time`}
						type="time"
						className={cn(CTL, 'w-[110px] font-mono text-[11.5px]')}
						value={schedule.time}
						onChange={(event) => onChange({ time: event.target.value })}
					/>
				</Fr>
			) : null}
			{frequency === 'cron' ? (
				<Fr
					label={<label htmlFor={`${prefix}-cron`}>Expression cron</label>}
					hint="5 champs : minute heure jour mois jour-semaine. Ex. 30 6 * * 1-5 : 6 h 30 en semaine."
				>
					<input
						id={`${prefix}-cron`}
						className={cn(CTL, 'font-mono text-[11.5px]')}
						placeholder="30 6 * * 1-5"
						value={schedule.cron}
						onChange={(event) => onChange({ cron: event.target.value })}
					/>
				</Fr>
			) : null}
		</>
	)
}
