export type FieldSize = 'md' | 'sm'

export const FIELD_BOX =
	'group/box flex w-full min-w-0 items-center gap-2 rounded-md border border-input bg-card transition-[color,box-shadow,border-color] duration-150 focus-within:border-ring focus-within:ring-[3px] focus-within:ring-ring/25 data-[invalid=true]:border-destructive data-[invalid=true]:ring-destructive/20 data-[disabled=true]:cursor-not-allowed data-[disabled=true]:opacity-50'

export const FIELD_PADDING: Record<FieldSize, string> = {
	md: 'min-h-8 px-3',
	sm: 'min-h-7 px-2.5',
}

export const FIELD_TEXT: Record<FieldSize, string> = {
	md: 'text-sm',
	sm: 'text-[13px]',
}

export const FIELD_SLOT =
	'flex shrink-0 items-center text-muted-foreground [&_svg]:pointer-events-none [&_svg:not([class*=size-])]:size-4'

export const FIELD_ICON_BUTTON =
	'flex size-5 shrink-0 items-center justify-center rounded-sm text-muted-foreground transition-colors duration-150 hover:bg-accent hover:text-accent-foreground focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring disabled:pointer-events-none'
