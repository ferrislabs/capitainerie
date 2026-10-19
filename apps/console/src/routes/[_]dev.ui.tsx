import { createFileRoute, notFound } from '@tanstack/react-router'
import { UiShowcase } from '#/dev/ui-showcase'

export const Route = createFileRoute('/_dev/ui')({
	beforeLoad: () => {
		if (!import.meta.env.DEV) throw notFound()
	},
	component: UiShowcase,
})
