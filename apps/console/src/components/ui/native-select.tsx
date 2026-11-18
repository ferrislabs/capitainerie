import type { ComponentProps } from 'react'
import { FIELD_BOX, FIELD_PADDING, FIELD_TEXT } from '#/components/ui/field-box'
import { cn } from '#/lib/utils'

/** Liste déroulante native au style des champs compacts, pour les formulaires. */
export function NativeSelect({
	className,
	...props
}: ComponentProps<'select'>) {
	return (
		<select
			className={cn(
				FIELD_BOX,
				FIELD_PADDING.md,
				FIELD_TEXT.md,
				'h-7 appearance-auto',
				className,
			)}
			{...props}
		/>
	)
}
