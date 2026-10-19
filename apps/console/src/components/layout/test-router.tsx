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
	'/runs',
	'/live',
	'/kanban',
	'/agents',
	'/settings',
	'/settings/providers',
	'/settings/models',
	'/settings/integrations',
] as const

/** Monte `ui` dans un routeur mémoire positionné sur `path`. */
export async function renderAtPath(ui: ReactNode, path: string) {
	const rootRoute = createRootRoute({
		component: () => (
			<>
				{ui}
				<Outlet />
			</>
		),
	})
	const routes = PATHS.map((routePath) =>
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
	return render(<RouterProvider router={router} />)
}
