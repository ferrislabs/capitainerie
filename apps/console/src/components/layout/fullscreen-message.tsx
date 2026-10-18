import type { ReactNode } from 'react'
import {
	CapitainerieMark,
	CapitainerieWordmark,
} from '#/components/brand/capitainerie-mark'

interface Props {
	icon?: ReactNode
	title: string
	message?: string
	action?: ReactNode
}

export function FullscreenMessage({ icon, title, message, action }: Props) {
	return (
		<main className="flex min-h-dvh flex-col bg-background">
			<div className="flex h-14 items-center gap-2.5 bg-rail px-5 text-rail-foreground-active">
				<CapitainerieMark className="size-7" />
				<CapitainerieWordmark />
			</div>
			<div className="flex flex-1 items-center justify-center p-6">
				<div className="flex w-full max-w-md flex-col items-center gap-4 rounded-lg border bg-card px-8 py-10 text-center">
					{icon ? (
						<span className="flex size-11 items-center justify-center rounded-md bg-secondary text-secondary-foreground">
							{icon}
						</span>
					) : null}
					<div className="flex flex-col gap-1.5">
						<p className="text-base font-semibold">{title}</p>
						{message ? (
							<p className="text-[13px] break-words text-muted-foreground">
								{message}
							</p>
						) : null}
					</div>
					{action}
				</div>
			</div>
		</main>
	)
}
