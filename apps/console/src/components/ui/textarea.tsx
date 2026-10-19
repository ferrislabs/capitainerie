import {
	type ChangeEvent,
	type ComponentProps,
	useLayoutEffect,
	useRef,
	useState,
} from 'react'

import { Counter, useFieldControl } from '#/components/ui/field'
import { cn } from '#/lib/utils'

interface TextareaProps extends ComponentProps<'textarea'> {
	autoGrow?: boolean
	maxRows?: number
	showCount?: boolean
	invalid?: boolean
}

const FALLBACK_LINE_HEIGHT = 20

function px(value: string): number {
	const parsed = Number.parseFloat(value)
	return Number.isNaN(parsed) ? 0 : parsed
}

function fitHeight(element: HTMLTextAreaElement, maxRows: number | undefined) {
	const style = window.getComputedStyle(element)
	const lineHeight = px(style.lineHeight) || FALLBACK_LINE_HEIGHT
	const border = px(style.borderTopWidth) + px(style.borderBottomWidth)
	const padding = px(style.paddingTop) + px(style.paddingBottom)
	element.style.height = 'auto'
	const wanted = element.scrollHeight + border
	const ceiling =
		maxRows === undefined ? undefined : lineHeight * maxRows + padding + border
	const clamped = ceiling !== undefined && wanted > ceiling
	element.style.height = `${clamped ? ceiling : wanted}px`
	element.style.overflowY = clamped ? 'auto' : 'hidden'
}

function Textarea({
	autoGrow = false,
	maxRows,
	showCount = false,
	invalid,
	className,
	id,
	value,
	defaultValue,
	onChange,
	disabled,
	required,
	maxLength,
	rows,
	ref,
	'aria-describedby': describedBy,
	'aria-invalid': ariaInvalid,
	...props
}: TextareaProps) {
	const control = useFieldControl({
		id,
		'aria-describedby': describedBy,
		'aria-invalid': ariaInvalid,
		required,
		disabled,
	})
	const inner = useRef<HTMLTextAreaElement | null>(null)
	const [typed, setTyped] = useState(String(defaultValue ?? ''))
	const current = value === undefined ? typed : String(value)
	const isInvalid = invalid === true || control.invalid
	const counterId = `${control.id ?? 'textarea'}-count`
	const grows = autoGrow || maxRows !== undefined

	useLayoutEffect(() => {
		const element = inner.current
		if (element && grows) fitHeight(element, maxRows)
	}, [current, grows, maxRows])

	const setRefs = (node: HTMLTextAreaElement | null) => {
		inner.current = node
		if (typeof ref === 'function') ref(node)
		else if (ref) ref.current = node
	}

	const handleChange = (event: ChangeEvent<HTMLTextAreaElement>) => {
		setTyped(event.target.value)
		onChange?.(event)
	}

	const area = (
		<textarea
			ref={setRefs}
			data-slot="textarea"
			id={control.id}
			value={value}
			defaultValue={defaultValue}
			onChange={handleChange}
			disabled={control.disabled}
			required={required}
			maxLength={maxLength}
			rows={rows}
			aria-required={control.required && !required ? true : undefined}
			aria-invalid={isInvalid || undefined}
			aria-describedby={
				showCount && maxLength !== undefined
					? [control.describedBy, counterId].filter(Boolean).join(' ')
					: control.describedBy
			}
			className={cn(
				'flex min-h-16 w-full resize-y rounded-md border border-input bg-card px-3 py-1.5 text-sm transition-[color,box-shadow,border-color] duration-150 outline-none placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-[3px] focus-visible:ring-ring/25 disabled:cursor-not-allowed disabled:opacity-50 aria-invalid:border-destructive aria-invalid:ring-destructive/20',
				grows ? 'resize-none' : 'field-sizing-content',
				className,
			)}
			{...props}
		/>
	)

	if (!showCount) return area

	return (
		<div className="flex flex-col gap-1">
			{area}
			<div className="flex justify-end">
				<Counter id={counterId} count={current.length} max={maxLength} />
			</div>
		</div>
	)
}

export { Textarea, type TextareaProps }
