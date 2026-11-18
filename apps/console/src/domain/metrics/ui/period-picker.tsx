import { DateInput } from '#/components/ui/date-input'
import { NativeSelect } from '#/components/ui/native-select'
import { type CustomRange, PERIODS, type Period } from '#/domain/metrics/model'

interface Props {
	period: Period
	range: CustomRange
	invalid: boolean
	onPeriod: (period: Period) => void
	onRange: (range: CustomRange) => void
}

export function PeriodPicker({
	period,
	range,
	invalid,
	onPeriod,
	onRange,
}: Props) {
	return (
		<>
			<NativeSelect
				aria-label="Période"
				value={period}
				onChange={(e) => onPeriod(e.target.value as Period)}
				className="w-auto"
			>
				{PERIODS.map((item) => (
					<option key={item.value} value={item.value}>
						{item.label}
					</option>
				))}
			</NativeSelect>
			{period === 'libre' ? (
				<>
					<DateInput
						aria-label="Du"
						value={range.from}
						max={range.to || undefined}
						aria-invalid={invalid || undefined}
						onChange={(e) => onRange({ ...range, from: e.target.value })}
					/>
					<DateInput
						aria-label="Au (inclus)"
						value={range.to}
						min={range.from || undefined}
						aria-invalid={invalid || undefined}
						onChange={(e) => onRange({ ...range, to: e.target.value })}
					/>
				</>
			) : null}
		</>
	)
}
