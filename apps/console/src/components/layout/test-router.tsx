import {
	createMemoryHistory,
	createRootRoute,
	createRoute,
	createRouter,
	Outlet,
	RouterProvider,
} from '@tanstack/react-router'
import { render } from '@testing-library/react'
import type { ReactNode } from 'react'

const PATHS = [
	'/',
	'/chat',
	'/chat/$id',
	'/runs',
	'/runs/$runId',
	'/live',
	'/kanban',
	'/clients',
	'/projects',
	'/worksites',
	'/agents',
	'/routines',
	'/tasks',
	'/tasks/$taskId',
	'/dashboard',
	'/agents/$agentId/{-$tab}',
	'/sources',
	'/skills',
	'/approvals',
	'/artifacts',
	'/artifacts/$artifactId',
	'/inbox/{-$tab}',
	'/connectors/{-$tab}',
	'/providers',
	'/audit/{-$tab}',
	'/settings',
	'/settings/providers',
	'/settings/models',
	'/settings/integrations',
	'/settings/integrations/github',
] as const

/** Monte `ui` dans un routeur mémoire positionné sur `path`. */
export async function renderAtPath(
	ui: ReactNode,
	path: string,
	extraPaths: ReadonlyArray<string> = [],
) {
	const rootRoute = createRootRoute({
		component: () => (
			<>
				{ui}
				<Outlet />
			</>
		),
	})
	const routes = [...new Set<string>([...PATHS, ...extraPaths])].map(
		(routePath) =>
			createRoute({
				getParentRoute: () => rootRoute,
				path: routePath,
				component: () => null,
			}),
	)
	const router = createRouter({
		routeTree: rootRoute.addChildren(routes),
		history: createMemoryHistory({ initialEntries: [path] }),
	})
	await router.load()
	return Object.assign(render(<RouterProvider router={router} />), { router })
}
