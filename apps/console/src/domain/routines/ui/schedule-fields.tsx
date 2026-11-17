import { useId } from 'react'
import { Checkbox } from '#/components/ui/checkbox'
import { Field } from '#/components/ui/field'
import { TextInput } from '#/components/ui/text-input'
import { toggleDay } from '#/domain/routines/routine-draft'
import {
	DAYS,
	FREQUENCIES,
	type Frequency,
	type Schedule,
} from '#/domain/routines/schedule'
import { SelectField } from '#/domain/routines/ui/select-field'

interface Props {
	schedule: Schedule
	onChange: (patch: Partial<Schedule>) => void
}

const isFrequency = (value: string): value is Frequency =>
	FREQUENCIES.some((f) => f === value)

const label = (day: string) => `${day.charAt(0).toUpperCase()}${day.slice(1)}.`

export function ScheduleFields({ schedule, onChange }: Props) {
	const prefix = useId()
	const { frequency } = schedule
	return (
		<div className="flex max-w-md flex-col gap-3">
			<SelectField
				label="Fréquence"
				value={frequency}
				options={FREQUENCIES}
				onChange={(value) => {
					if (isFrequency(value)) onChange({ frequency: value })
				}}
			/>
			{frequency === 'Toutes les N heures' ? (
				<Field label="Toutes les (heures)">
					<TextInput
						type="number"
						min={1}
						max={168}
						value={schedule.hours}
						onChange={(event) =>
							onChange({ hours: Math.max(1, Number(event.target.value) || 1) })
						}
					/>
				</Field>
			) : null}
			{frequency === 'Toutes les semaines' ? (
				<fieldset className="m-0 flex min-w-0 flex-col gap-1.5 border-0 p-0">
					<legend className="mb-1.5 p-0 text-sm leading-none font-medium">
						Jours
					</legend>
					<div className="flex flex-wrap gap-x-4 gap-y-1">
						{DAYS.map((day) => (
							<Checkbox
								key={day}
								id={`${prefix}-${day}`}
								label={label(day)}
								checked={schedule.days.includes(day)}
								onCheckedChange={() =>
									onChange({ days: toggleDay(schedule.days, day) })
								}
							/>
						))}
					</div>
				</fieldset>
			) : null}
			{frequency !== 'Toutes les N heures' &&
			frequency !== 'Expression cron' ? (
				<Field label="Heure">
					<TextInput
						className="font-mono"
						placeholder="08:00"
						value={schedule.time}
						onChange={(event) => onChange({ time: event.target.value })}
					/>
				</Field>
			) : null}
			{frequency === 'Expression cron' ? (
				<Field
					label="Expression cron"
					hint="minute heure jour mois jour-semaine"
				>
					<TextInput
						className="font-mono"
						placeholder="30 6 * * 1"
						value={schedule.cron}
						onChange={(event) => onChange({ cron: event.target.value })}
					/>
				</Field>
			) : null}
		</div>
	)
}
