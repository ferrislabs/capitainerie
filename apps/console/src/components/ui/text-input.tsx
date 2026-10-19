import { Loader2, X } from 'lucide-react'
import {
	type ChangeEvent,
	type ComponentProps,
	type MouseEvent,
	type ReactNode,
	useRef,
	useState,
} from 'react'

import { useFieldControl } from '#/components/ui/field'
import {
	FIELD_BOX,
	FIELD_ICON_BUTTON,
	FIELD_PADDING,
	FIELD_SLOT,
	FIELD_TEXT,
	type FieldSize,
} from '#/components/ui/field-box'
import { cn } from '#/lib/utils'

type TextInputSize = FieldSize

interface TextInputProps extends Omit<ComponentProps<'input'>, 'size'> {
	size?: TextInputSize
	leading?: ReactNode
	trailing?: ReactNode
	clearable?: boolean
	clearLabel?: string
	loading?: boolean
	invalid?: boolean
	inputClassName?: string
}

function clearNativeValue(input: HTMLInputElement) {
	const setter = Object.getOwnPropertyDescriptor(
		HTMLInputElement.prototype,
		'value',
	)?.set
	setter?.call(input, '')
	input.dispatchEvent(new Event('input', { bubbles: true }))
}

function TextInput({
	size = 'md',
	leading,
	trailing,
	clearable = false,
	clearLabel = 'Effacer',
	loading = false,
	invalid,
	className,
	inputClassName,
	id,
	value,
	defaultValue,
	onChange,
	disabled,
	readOnly,
	required,
	ref,
	'aria-describedby': describedBy,
	'aria-invalid': ariaInvalid,
	...props
}: TextInputProps) {
	const control = useFieldControl({
		id,
		'aria-describedby': describedBy,
		'aria-invalid': ariaInvalid,
		required,
		disabled,
	})
	const inner = useRef<HTMLInputElement | null>(null)
	const [typed, setTyped] = useState(String(defaultValue ?? ''))
	const current = value === undefined ? typed : String(value)
	const isInvalid = invalid === true || control.invalid
	const canClear =
		clearable && current !== '' && !control.disabled && readOnly !== true

	const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
		setTyped(event.target.value)
		onChange?.(event)
	}

	const focusInput = (event: MouseEvent<HTMLDivElement>) => {
		if (event.target === event.currentTarget) {
			event.preventDefault()
			inner.current?.focus()
		}
	}

	const setRefs = (node: HTMLInputElement | null) => {
		inner.current = node
		if (typeof ref === 'function') ref(node)
		else if (ref) ref.current = node
	}

	const clear = () => {
		if (inner.current) {
			clearNativeValue(inner.current)
			inner.current.focus()
		}
	}

	return (
		<div
			data-slot="text-input"
			data-size={size}
			data-invalid={isInvalid}
			data-disabled={control.disabled}
			onMouseDown={focusInput}
			className={cn(
				FIELD_BOX,
				FIELD_PADDING[size],
				FIELD_TEXT[size],
				className,
			)}
		>
			{leading ? <span className={FIELD_SLOT}>{leading}</span> : null}
			<input
				ref={setRefs}
				id={control.id}
				value={value}
				defaultValue={defaultValue}
				onChange={handleChange}
				disabled={control.disabled}
				readOnly={readOnly}
				required={required}
				aria-required={control.required && !required ? true : undefined}
				aria-invalid={isInvalid || undefined}
				aria-describedby={control.describedBy}
				aria-busy={loading || undefined}
				className={cn(
					'h-full min-h-0 min-w-0 flex-1 self-stretch bg-transparent py-1 outline-none selection:bg-primary selection:text-primary-foreground placeholder:text-muted-foreground disabled:cursor-not-allowed',
					inputClassName,
				)}
				{...props}
			/>
			{loading ? (
				<span role="status" className={FIELD_SLOT}>
					<Loader2 aria-hidden="true" className="animate-spin" />
					<span className="sr-only">Chargement…</span>
				</span>
			) : null}
			{canClear ? (
				<button
					type="button"
					aria-label={clearLabel}
					onClick={clear}
					className={FIELD_ICON_BUTTON}
				>
					<X aria-hidden="true" className="size-3.5" />
				</button>
			) : null}
			{trailing ? <span className={FIELD_SLOT}>{trailing}</span> : null}
		</div>
	)
}

export { TextInput, type TextInputProps, type TextInputSize }
