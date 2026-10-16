import { Link } from '@tanstack/react-router'
import { Anchor, Bot, KanbanSquare, LogOut, Play } from 'lucide-react'
import { useAuth } from 'react-oidc-context'
import { Button } from '#/components/ui/button'
import { UserBadgeFeature } from '#/domain/identity/feature/user-badge-feature'

const NAV_ITEMS = [
	{ to: '/runs', label: 'Runs', icon: Play },
	{ to: '/kanban', label: 'Kanban', icon: KanbanSquare },
	{ to: '/agents', label: 'Agents', icon: Bot },
] as const

export function AppSidebar() {
	const auth = useAuth()

	return (
		<aside className="flex w-56 shrink-0 flex-col gap-6 border-r border-sidebar-border bg-sidebar p-4 text-sidebar-foreground">
			<div className="flex items-center gap-2 font-semibold">
				<Anchor className="size-5" aria-hidden="true" />
				Capitainerie
			</div>
			<nav aria-label="Navigation principale" className="flex flex-col gap-1">
				{NAV_ITEMS.map(({ to, label, icon: Icon }) => (
					<Link
						key={to}
						to={to}
						className="flex items-center gap-2 px-3 py-2 text-sm text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
						activeProps={{
							className: 'bg-sidebar-accent text-sidebar-accent-foreground',
						}}
					>
						<Icon className="size-4" aria-hidden="true" />
						{label}
					</Link>
				))}
			</nav>
			<div className="mt-auto flex flex-col gap-3 border-t border-sidebar-border pt-4">
				<UserBadgeFeature />
				<Button
					variant="outline"
					size="sm"
					onClick={() => {
						void auth.signoutRedirect()
					}}
				>
					<LogOut aria-hidden="true" />
					Se déconnecter
				</Button>
			</div>
		</aside>
	)
}
