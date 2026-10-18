import type { ReactNode } from 'react'

interface Props {
	organization: ReactNode
	actions: ReactNode
}

export function TopBar({ organization, actions }: Props) {
	return (
		<header className="sticky top-0 z-30 flex h-12 shrink-0 items-center justify-between gap-4 border-b bg-background/92 px-4 backdrop-blur-sm supports-[backdrop-filter]:bg-background/80 sm:px-6">
			<div className="min-w-0">{organization}</div>
			<div className="flex shrink-0 items-center gap-2">{actions}</div>
		</header>
	)
}
