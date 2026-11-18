import { type FormEvent, useState } from 'react'
import { InlineAlert } from '#/components/layout/inline-alert'
import { Button } from '#/components/ui/button'
import { type ScheduleDraft, scheduleError } from '#/domain/routines/schedule'
import { ScheduleFields } from '#/domain/routines/ui/schedule-fields'

interface Props {
	initial: ScheduleDraft
	submitLabel: string
	pending: boolean
	error: string | null
	onSubmit: (schedule: ScheduleDraft) => void
	onCancel: () => void
}

export function ScheduleEditor({
	initial,
	submitLabel,
	pending,
	error,
	onSubmit,
	onCancel,
}: Props) {
	const [schedule, setSchedule] = useState(initial)
	const [invalid, setInvalid] = useState<string | null>(null)
	const submit = (event: FormEvent) => {
		event.preventDefault()
		const problem = scheduleError(schedule)
		setInvalid(problem)
		if (!problem) onSubmit(schedule)
	}
	return (
		<form aria-label="Planning" onSubmit={submit} noValidate>
			<ScheduleFields
				schedule={schedule}
				onChange={(patch) => setSchedule((s) => ({ ...s, ...patch }))}
			/>
			<div className="grid gap-2 border-t px-3 py-2">
				{invalid || error ? (
					<InlineAlert tone="danger">{invalid ?? error}</InlineAlert>
				) : null}
				<div className="flex justify-end gap-2">
					<Button type="button" size="sm" variant="outline" onClick={onCancel}>
						Annuler
					</Button>
					<Button type="submit" size="sm" disabled={pending}>
						{pending ? 'Enregistrement…' : submitLabel}
					</Button>
				</div>
			</div>
		</form>
	)
}
