import { CircleAlert } from 'lucide-react'
import { createContext, type ReactNode, useContext, useId } from 'react'

import { Label } from '#/components/ui/label'
import { cn } from '#/lib/utils'

interface FieldControl {
	id: string
	labelId: string
	describedBy: string | undefined
	invalid: boolean
	required: boolean
	disabled: boolean
	group: boolean
}

const FieldContext = createContext<FieldControl | null>(null)

function joinIds(
	...ids: ReadonlyArray<string | undefined>
): string | undefined {
	const found = ids.filter((id): id is string => Boolean(id))
	return found.length > 0 ? found.join(' ') : undefined
}

interface ControlProps {
	id?: string | undefined
	'aria-describedby'?: string | undefined
	'aria-invalid'?:
		| boolean
		| 'true'
		| 'false'
		| 'grammar'
		| 'spelling'
		| undefined
	required?: boolean | undefined
	disabled?: boolean | undefined
}

function useFieldControl(props: ControlProps = {}) {
	const field = useContext(FieldContext)
	const own = props['aria-invalid']
	return {
		id: field?.id ?? props.id,
		labelId: field?.labelId,
		describedBy: joinIds(props['aria-describedby'], field?.describedBy),
		invalid: own === true || own === 'true' || field?.invalid === true,
		required: props.required ?? field?.required ?? false,
		disabled: props.disabled ?? field?.disabled ?? false,
		inField: field !== null,
	}
}

interface FieldProps {
	label: ReactNode
	id?: string
	hint?: ReactNode
	error?: ReactNode
	required?: boolean
	optional?: boolean
	disabled?: boolean
	count?: number
	max?: number
	group?: boolean
	hideLabel?: boolean
	labelClassName?: string
	className?: string
	children: ReactNode
}

function Counter({
	id,
	count,
	max,
}: {
	id: string
	count: number
	max: number | undefined
}) {
	const over = max !== undefined && count > max
	return (
		<span
			id={id}
			data-slot="field-counter"
			data-over={over}
			className={cn(
				'figures shrink-0 text-xs text-muted-foreground',
				over && 'font-medium text-signal-danger',
			)}
		>
			{max === undefined ? count : `${count} / ${max}`}
			{over ? ' · trop long' : null}
		</span>
	)
}

function Field({
	label,
	id,
	hint,
	error,
	required = false,
	optional = false,
	disabled = false,
	count,
	max,
	group = false,
	hideLabel = false,
	labelClassName,
	className,
	children,
}: FieldProps) {
	const generated = useId()
	const controlId = id ?? `${generated}-control`
	const labelId = `${generated}-label`
	const hintId = `${generated}-hint`
	const errorId = `${generated}-error`
	const counterId = `${generated}-counter`
	const hasError = Boolean(error)
	const hasCounter = count !== undefined

	const value: FieldControl = {
		id: controlId,
		labelId,
		describedBy: joinIds(
			hasError ? errorId : undefined,
			hint ? hintId : undefined,
			hasCounter ? counterId : undefined,
		),
		invalid: hasError,
		required,
		disabled,
		group,
	}

	const marker = required ? (
		<>
			<span aria-hidden="true" className="text-muted-foreground">
				*
			</span>
			<span className="sr-only">(obligatoire)</span>
		</>
	) : optional ? (
		<span className="font-normal text-muted-foreground">(facultatif)</span>
	) : null

	const caption = (
		<>
			{label}
			{marker}
		</>
	)

	return (
		<FieldContext.Provider value={value}>
			<div
				data-slot="field"
				data-invalid={hasError}
				role={group ? 'group' : undefined}
				aria-labelledby={group ? labelId : undefined}
				aria-describedby={group ? value.describedBy : undefined}
				className={cn('flex flex-col gap-1.5', className)}
			>
				<div className="flex items-baseline justify-between gap-2">
					{group ? (
						<span
							id={labelId}
							className={cn(
								'flex items-center gap-1 text-sm leading-none font-medium',
								hideLabel && 'sr-only',
								labelClassName,
							)}
						>
							{caption}
						</span>
					) : (
						<Label
							id={labelId}
							htmlFor={controlId}
							className={cn('gap-1', hideLabel && 'sr-only', labelClassName)}
						>
							{caption}
						</Label>
					)}
					{hasCounter ? (
						<Counter id={counterId} count={count} max={max} />
					) : null}
				</div>
				{children}
				{hint ? (
					<p id={hintId} className="text-xs text-muted-foreground">
						{hint}
					</p>
				) : null}
				{hasError ? (
					<p
						id={errorId}
						role="alert"
						data-slot="field-error"
						className="flex items-start gap-1.5 text-xs font-medium text-destructive"
					>
						<CircleAlert
							aria-hidden="true"
							className="mt-px size-3.5 shrink-0"
						/>
						<span>{error}</span>
					</p>
				) : null}
			</div>
		</FieldContext.Provider>
	)
}

export { Counter, Field, joinIds, useFieldControl }
