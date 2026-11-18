import { createFileRoute, redirect } from '@tanstack/react-router'
import { MetricsPage } from '#/domain/metrics/feature/metrics-page'
import { parseMetricsSearch } from '#/domain/metrics/search'
import { parseMetricsTab } from '#/domain/metrics/tabs'

export const Route = createFileRoute('/_app/metrics/{-$tab}')({
	validateSearch: parseMetricsSearch,
	beforeLoad: ({ params, search }) => {
		if (params.tab === undefined)
			throw redirect({
				to: '/metrics/{-$tab}',
				params: { tab: 'projets' },
				search,
				replace: true,
			})
	},
	component: MetricsRoute,
})

function MetricsRoute() {
	const { tab } = Route.useParams()
	return <MetricsPage tab={parseMetricsTab(tab)} />
}
