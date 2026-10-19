import { Check, ChevronDown, Loader2, X } from 'lucide-react'
import { Popover as PopoverPrimitive } from 'radix-ui'
import {
	type KeyboardEvent,
	type MouseEvent,
	useEffect,
	useId,
	useRef,
	useState,
} from 'react'

import {
	allOptions,
	buildSections,
	type ComboboxItem,
	type ComboboxOption,
	edgeIndex,
	flattenSections,
	stepIndex,
	typeaheadIndex,
} from '#/components/ui/combobox-model'
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

interface CommonProps {
	items: ReadonlyArray<ComboboxItem>
	placeholder?: string
	emptyText?: string
	loadingText?: string
	noneLabel?: string
	loading?: boolean
	disabled?: boolean
	invalid?: boolean
	required?: boolean
	clearable?: boolean
	searchable?: boolean
	size?: FieldSize
	id?: string
	className?: string
	listClassName?: string
	'aria-label'?: string
	'aria-describedby'?: string
	'aria-invalid'?: boolean | 'true' | 'false'
}

interface SingleProps extends CommonProps {
	multiple?: false
	value: string
	onValueChange: (value: string) => void
}

interface MultiProps extends CommonProps {
	multiple: true
	value: ReadonlyArray<string>
	onValueChange: (value: string[]) => void
}

type ComboboxProps = SingleProps | MultiProps

const TYPEAHEAD_RESET_MS = 500

const ROW =
	'relative flex cursor-default items-center gap-2 rounded-md py-1.5 pr-8 pl-2 select-none data-[active=true]:bg-accent data-[active=true]:text-accent-foreground data-[disabled=true]:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*=size-])]:size-4'

function Combobox(props: ComboboxProps) {
	const {
		items,
		placeholder,
		emptyText = 'Aucun résultat',
		loadingText = 'Chargement…',
		noneLabel,
		loading = false,
		clearable = false,
		size = 'md',
		className,
		listClassName,
	} = props
	const multiple = props.multiple === true
	const editable = multiple || (props.searchable ?? true)
	const control = useFieldControl({
		id: props.id,
		'aria-describedby': props['aria-describedby'],
		'aria-invalid': props['aria-invalid'],
		required: props.required,
		disabled: props.disabled,
	})
	const uid = useId()
	const controlId = control.id ?? `${uid}-control`
	const listId = `${uid}-list`
	const optionId = (index: number) => `${uid}-opt-${index}`
	const anchorRef = useRef<HTMLDivElement | null>(null)
	const inputRef = useRef<HTMLInputElement | null>(null)
	const typeBuffer = useRef('')
	const typeTimer = useRef<number | undefined>(undefined)
	const [open, setOpen] = useState(false)
	const [query, setQuery] = useState<string | null>(null)
	const [activeValue, setActiveValue] = useState<string | null>(null)

	const selectedValues: ReadonlyArray<string> = props.multiple
		? props.value
		: [props.value]
	const everyOption = allOptions(items)
	const labelOf = (value: string) =>
		everyOption.find((option) => option.value === value)?.label ?? value
	const noneOption: ComboboxOption | undefined =
		!multiple && noneLabel !== undefined
			? { value: '', label: noneLabel }
			: undefined

	const optionsFor = (text: string) => {
		const sections = buildSections(items, text)
		const base = flattenSections(sections)
		const flat = noneOption && text.trim() === '' ? [noneOption, ...base] : base
		return { sections, flat }
	}

	const { sections, flat } = optionsFor(editable ? (query ?? '') : '')
	const indexByValue = new Map(
		flat.map((option, index) => [option.value, index]),
	)
	const activeIndex =
		open && activeValue !== null ? (indexByValue.get(activeValue) ?? -1) : -1
	const hasValue = props.multiple ? props.value.length > 0 : props.value !== ''
	const disabled = control.disabled
	const isInvalid = props.invalid === true || control.invalid
	const selectedLabel = props.multiple
		? undefined
		: props.value === ''
			? placeholder === undefined
				? noneOption?.label
				: undefined
			: labelOf(props.value)
	const inputValue = query ?? selectedLabel ?? ''
	const canClear = clearable && hasValue && !disabled

	useEffect(() => {
		return () => window.clearTimeout(typeTimer.current)
	}, [])

	useEffect(() => {
		if (open && activeIndex >= 0) {
			document
				.getElementById(`${uid}-opt-${activeIndex}`)
				?.scrollIntoView?.({ block: 'nearest' })
		}
	}, [open, activeIndex, uid])

	const firstActive = (list: ReadonlyArray<ComboboxOption>) => {
		const chosen = list.find(
			(option) =>
				option.disabled !== true && selectedValues.includes(option.value),
		)
		const fallback = list[edgeIndex(list, 'first')]
		return (chosen ?? fallback)?.value ?? null
	}

	const openList = (edge?: 'last') => {
		if (disabled) return
		setOpen(true)
		if (edge === 'last' && flat.length > 0) {
			setActiveValue(flat[edgeIndex(flat, 'last')]?.value ?? null)
		} else {
			setActiveValue(firstActive(flat))
		}
	}

	const close = () => {
		setOpen(false)
		setQuery(null)
	}

	const choose = (option: ComboboxOption) => {
		if (option.disabled) return
		if (props.multiple) {
			const next = props.value.includes(option.value)
				? props.value.filter((value) => value !== option.value)
				: [...props.value, option.value]
			props.onValueChange(next)
			setQuery(null)
			setActiveValue(option.value)
			inputRef.current?.focus()
		} else {
			props.onValueChange(option.value)
			close()
		}
	}

	const removeValue = (value: string) => {
		if (props.multiple) {
			props.onValueChange(props.value.filter((item) => item !== value))
			inputRef.current?.focus()
		}
	}

	const clearAll = () => {
		if (props.multiple) props.onValueChange([])
		else props.onValueChange('')
		setQuery(null)
		inputRef.current?.focus()
	}

	const move = (direction: 1 | -1) => {
		const next =
			activeIndex < 0
				? edgeIndex(flat, direction === 1 ? 'first' : 'last')
				: stepIndex(flat, activeIndex, direction)
		const option = flat[next]
		if (option) setActiveValue(option.value)
	}

	const typeahead = (char: string) => {
		typeBuffer.current += char
		window.clearTimeout(typeTimer.current)
		typeTimer.current = window.setTimeout(() => {
			typeBuffer.current = ''
		}, TYPEAHEAD_RESET_MS)
		const from =
			activeIndex >= 0
				? activeIndex
				: flat.findIndex((option) => selectedValues.includes(option.value))
		const index = typeaheadIndex(flat, from, typeBuffer.current)
		const option = flat[index]
		if (option) {
			if (!open) setOpen(true)
			setActiveValue(option.value)
		}
	}

	const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
		if (disabled || event.nativeEvent.isComposing) return
		const { key } = event
		if (key === 'ArrowDown' || key === 'ArrowUp') {
			event.preventDefault()
			if (!open) openList(key === 'ArrowUp' ? 'last' : undefined)
			else move(key === 'ArrowDown' ? 1 : -1)
			return
		}
		if (key === 'Home' || key === 'End') {
			if (editable && (query ?? '') !== '') return
			event.preventDefault()
			if (!open) setOpen(true)
			const index = edgeIndex(flat, key === 'Home' ? 'first' : 'last')
			setActiveValue(flat[index]?.value ?? null)
			return
		}
		if (key === 'Enter' || (key === ' ' && !editable)) {
			if (key === ' ' && typeBuffer.current !== '') {
				event.preventDefault()
				typeahead(' ')
				return
			}
			if (open) {
				event.preventDefault()
				const option = flat[activeIndex]
				if (option) choose(option)
			} else if (!editable) {
				event.preventDefault()
				openList()
			}
			return
		}
		if (key === 'Escape') {
			if (open) {
				event.preventDefault()
				close()
			}
			return
		}
		if (key === 'Tab') {
			close()
			return
		}
		if (
			editable &&
			!multiple &&
			query === null &&
			inputValue !== '' &&
			(key === 'Backspace' ||
				key === 'Delete' ||
				(key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey))
		) {
			setQuery('')
			return
		}
		if (
			key === 'Backspace' &&
			props.multiple &&
			inputValue === '' &&
			props.value.length > 0
		) {
			props.onValueChange(props.value.slice(0, -1))
			return
		}
		if (
			!editable &&
			key.length === 1 &&
			!event.ctrlKey &&
			!event.metaKey &&
			!event.altKey
		) {
			event.preventDefault()
			typeahead(key)
		}
	}

	const onInputChange = (text: string) => {
		setQuery(text)
		setOpen(true)
		const first = optionsFor(text).flat
		setActiveValue(first[edgeIndex(first, 'first')]?.value ?? null)
	}

	const keepFocus = (event: MouseEvent<HTMLElement>) => event.preventDefault()

	const focusInput = (event: MouseEvent<HTMLDivElement>) => {
		if (event.target === event.currentTarget) {
			event.preventDefault()
			inputRef.current?.focus()
			if (!open) openList()
		}
	}

	const status = loading ? (
		<span role="status" className={FIELD_SLOT}>
			<Loader2 aria-hidden="true" className="animate-spin" />
			<span className="sr-only">{loadingText}</span>
		</span>
	) : null

	const clearButton = canClear ? (
		<button
			type="button"
			aria-label={multiple ? 'Tout effacer' : 'Effacer la sélection'}
			onMouseDown={keepFocus}
			onClick={clearAll}
			className={FIELD_ICON_BUTTON}
		>
			<X aria-hidden="true" className="size-3.5" />
		</button>
	) : null

	const sharedAria = {
		role: 'combobox' as const,
		'aria-haspopup': 'listbox' as const,
		'aria-expanded': open,
		'aria-controls': open ? listId : undefined,
		'aria-activedescendant':
			open && activeIndex >= 0 ? optionId(activeIndex) : undefined,
		'aria-invalid': isInvalid || undefined,
		'aria-required': control.required || undefined,
		'aria-describedby': control.describedBy,
		'aria-label': props['aria-label'],
	}

	const trigger = editable ? (
		<div
			ref={anchorRef}
			data-slot="combobox"
			data-invalid={isInvalid}
			data-disabled={disabled}
			onMouseDown={focusInput}
			className={cn(
				FIELD_BOX,
				FIELD_PADDING[size],
				FIELD_TEXT[size],
				'flex-wrap py-1',
				className,
			)}
		>
			{props.multiple
				? props.value.map((value) => (
						<span
							key={value}
							data-slot="combobox-chip"
							className="inline-flex h-5 max-w-full items-center gap-1 rounded-sm bg-secondary pr-0.5 pl-1.5 text-xs text-secondary-foreground"
						>
							<span className="truncate">{labelOf(value)}</span>
							<button
								type="button"
								tabIndex={-1}
								disabled={disabled}
								aria-label={`Retirer ${labelOf(value)}`}
								onMouseDown={keepFocus}
								onClick={() => removeValue(value)}
								className="flex size-4 items-center justify-center rounded-sm text-muted-foreground hover:bg-accent hover:text-accent-foreground disabled:pointer-events-none"
							>
								<X aria-hidden="true" className="size-3" />
							</button>
						</span>
					))
				: null}
			<input
				{...sharedAria}
				ref={inputRef}
				id={controlId}
				type="text"
				autoComplete="off"
				aria-autocomplete="list"
				disabled={disabled}
				value={inputValue}
				placeholder={multiple && hasValue ? undefined : placeholder}
				onChange={(event) => onInputChange(event.target.value)}
				onKeyDown={onKeyDown}
				onClick={() => {
					if (!open) openList()
				}}
				onFocus={(event) => {
					if (!multiple) event.currentTarget.select()
				}}
				onBlur={close}
				className="h-6 min-w-16 flex-1 bg-transparent outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed"
			/>
			{status}
			{clearButton}
			<button
				type="button"
				tabIndex={-1}
				aria-hidden="true"
				disabled={disabled}
				onMouseDown={keepFocus}
				onClick={() => {
					if (open) close()
					else {
						inputRef.current?.focus()
						openList()
					}
				}}
				className={FIELD_ICON_BUTTON}
			>
				<ChevronDown aria-hidden="true" className="size-4" />
			</button>
		</div>
	) : (
		<div
			ref={anchorRef}
			data-slot="combobox"
			className={cn('relative w-full', FIELD_TEXT[size], className)}
		>
			<button
				{...sharedAria}
				type="button"
				id={controlId}
				data-invalid={isInvalid}
				data-disabled={disabled}
				disabled={disabled}
				onKeyDown={onKeyDown}
				onKeyUp={(event) => {
					if (event.key === ' ') event.preventDefault()
				}}
				onClick={() => (open ? close() : openList())}
				onBlur={close}
				className={cn(
					FIELD_BOX,
					FIELD_PADDING[size],
					'cursor-pointer justify-between text-left disabled:cursor-not-allowed',
					canClear ? 'pr-14' : 'pr-8',
				)}
			>
				<span
					className={cn(
						'min-w-0 flex-1 truncate',
						selectedLabel === undefined && 'text-muted-foreground',
					)}
				>
					{selectedLabel ?? placeholder ?? noneLabel ?? ''}
				</span>
			</button>
			<span className="pointer-events-none absolute inset-y-0 right-2.5 flex items-center gap-1 text-muted-foreground">
				{status}
				<ChevronDown aria-hidden="true" className="size-4" />
			</span>
			{canClear ? (
				<span className="absolute inset-y-0 right-8 flex items-center">
					{clearButton}
				</span>
			) : null}
		</div>
	)

	return (
		<PopoverPrimitive.Root
			open={open}
			onOpenChange={(next) => (next ? openList() : close())}
		>
			<PopoverPrimitive.Anchor asChild>{trigger}</PopoverPrimitive.Anchor>
			<PopoverPrimitive.Portal>
				<PopoverPrimitive.Content
					role="presentation"
					align="start"
					sideOffset={4}
					collisionPadding={8}
					onOpenAutoFocus={(event) => event.preventDefault()}
					onCloseAutoFocus={(event) => event.preventDefault()}
					onInteractOutside={(event) => {
						if (anchorRef.current?.contains(event.target as Node)) {
							event.preventDefault()
						}
					}}
					onMouseDown={keepFocus}
					style={{ width: 'var(--radix-popover-trigger-width)' }}
					className={cn(
						'z-50 min-w-40 overflow-hidden rounded-lg border bg-popover p-1 text-popover-foreground shadow-float outline-none',
						'data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-[0.98] data-[state=closed]:animate-out data-[state=closed]:fade-out-0',
						FIELD_TEXT[size],
					)}
				>
					<div
						id={listId}
						role="listbox"
						aria-multiselectable={multiple || undefined}
						aria-label={
							props['aria-label']
								? `${props['aria-label']} : options`
								: 'Options'
						}
						aria-busy={loading || undefined}
						onWheel={(event) => event.stopPropagation()}
						onTouchMove={(event) => event.stopPropagation()}
						className={cn(
							'max-h-64 overflow-y-auto overscroll-contain',
							listClassName,
						)}
					>
						{noneOption && flat[0] === noneOption ? (
							<OptionRow
								id={optionId(0)}
								option={noneOption}
								selected={selectedValues.includes('')}
								active={activeIndex === 0}
								onActivate={() => setActiveValue('')}
								onChoose={() => choose(noneOption)}
							/>
						) : null}
						{sections.map((section, sectionIndex) => {
							const rows = section.options.map((option) => {
								const index = indexByValue.get(option.value) ?? 0
								return (
									<OptionRow
										key={option.value}
										id={optionId(index)}
										option={option}
										selected={selectedValues.includes(option.value)}
										active={activeIndex === index}
										onActivate={() => setActiveValue(option.value)}
										onChoose={() => choose(option)}
									/>
								)
							})
							if (section.label === undefined) {
								return <div key={`loose-${sectionIndex}`}>{rows}</div>
							}
							const headerId = `${uid}-group-${sectionIndex}`
							return (
								<div
									key={section.label}
									role="group"
									aria-labelledby={headerId}
									className="[&:not(:first-child)]:mt-1"
								>
									<div
										id={headerId}
										role="presentation"
										className="px-2 pt-1.5 pb-1 text-xs font-medium text-muted-foreground"
									>
										{section.label}
									</div>
									{rows}
								</div>
							)
						})}
					</div>
					{loading ? (
						<p
							role="status"
							className="flex items-center gap-2 px-2 py-2 text-muted-foreground"
						>
							<Loader2 aria-hidden="true" className="size-4 animate-spin" />
							{loadingText}
						</p>
					) : flat.length === 0 ? (
						<p role="status" className="px-2 py-2 text-muted-foreground">
							{emptyText}
						</p>
					) : null}
				</PopoverPrimitive.Content>
			</PopoverPrimitive.Portal>
		</PopoverPrimitive.Root>
	)
}

function OptionRow({
	id,
	option,
	selected,
	active,
	onActivate,
	onChoose,
}: {
	id: string
	option: ComboboxOption
	selected: boolean
	active: boolean
	onActivate: () => void
	onChoose: () => void
}) {
	return (
		<div
			id={id}
			role="option"
			aria-selected={selected}
			aria-disabled={option.disabled || undefined}
			data-slot="combobox-option"
			data-value={option.value}
			data-active={active}
			data-disabled={option.disabled === true}
			onMouseMove={onActivate}
			onClick={onChoose}
			className={ROW}
		>
			{option.icon}
			<span className="flex min-w-0 flex-1 flex-col">
				<span className="truncate">{option.label}</span>
				{option.description ? (
					<span className="truncate text-xs text-muted-foreground">
						{option.description}
					</span>
				) : null}
			</span>
			{selected ? (
				<Check
					aria-hidden="true"
					className="absolute top-1/2 right-2 size-4 -translate-y-1/2"
				/>
			) : null}
		</div>
	)
}

type SelectProps = ComboboxProps

function Select(props: SelectProps) {
	return <Combobox searchable={false} {...props} />
}

export { Combobox, Select, type ComboboxProps, type SelectProps }
