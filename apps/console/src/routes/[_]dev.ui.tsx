import { createFileRoute, notFound } from '@tanstack/react-router'
import { lazy, Suspense } from 'react'

const Showcase = import.meta.env.DEV
	? lazy(() =>
			import('#/dev/ui-showcase').then((module) => ({
				default: module.UiShowcase,
			})),
		)
	: null

function DevUi() {
	if (!Showcase) return null
	return (
		<Suspense fallback={null}>
			<Showcase />
		</Suspense>
	)
}

export const Route = createFileRoute('/_dev/ui')({
	beforeLoad: () => {
		if (!import.meta.env.DEV) throw notFound()
	},
	component: DevUi,
})
