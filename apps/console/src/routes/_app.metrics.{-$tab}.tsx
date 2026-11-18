import { createFileRoute, redirect } from '@tanstack/react-router'
import { MetricsPage } from '#/domain/metrics/feature/metrics-page'
import { parseMetricsTab } from '#/domain/metrics/tabs'

export const Route = createFileRoute('/_app/metrics/{-$tab}')({
	beforeLoad: ({ params }) => {
		if (params.tab === undefined)
			throw redirect({
				to: '/metrics/{-$tab}',
				params: { tab: 'projets' },
				replace: true,
			})
	},
	component: MetricsRoute,
})

function MetricsRoute() {
	const { tab } = Route.useParams()
	return <MetricsPage tab={parseMetricsTab(tab)} />
}
