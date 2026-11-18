import { Archive, MoreHorizontal, Pause, Play, Plus } from 'lucide-react'
import { Button } from '#/components/ui/button'
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from '#/components/ui/dropdown-menu'
import type { TeamAgent } from '#/domain/agent-team/team-agent'
import { AdapterChip } from '#/domain/agent-team/ui/adapter-chip'
import { Mascot, type MascotState } from '#/domain/agent-team/ui/mascot'

interface Props {
	agent: TeamAgent
	adapter: string
	title: string
	paused: boolean
	mood?: MascotState
	busy?: boolean
	onAssign: () => void
	onTogglePause: () => void
	onToggleArchive: () => void
}

const ACTION = 'h-9 gap-2 px-3.5 text-[14px]'

export function AgentHeader({
	agent,
	adapter,
	title,
	paused,
	mood = 'idle',
	busy = false,
	onAssign,
	onTogglePause,
	onToggleArchive,
}: Props) {
	const meta = title || agent.description
	return (
		<header className="flex flex-col gap-7">
			<div className="flex min-w-0 items-center gap-6">
				<Mascot
					name={agent.name}
					hue={agent.hue}
					state={mood}
					className="h-[92px] w-[109px] shrink-0 max-sm:h-[68px] max-sm:w-[80px]"
				/>
				<div className="flex min-w-0 flex-col gap-2">
					<h1 className="m-0 text-[28px] leading-[1.15] font-semibold tracking-[-0.02em] [overflow-wrap:anywhere] max-sm:text-[22px]">
						{agent.name}
						{agent.archived ? (
							<span className="ml-3 inline-block rounded-full border px-2.5 py-0.5 align-middle text-xs font-medium tracking-normal text-muted-foreground">
								Archivé
							</span>
						) : null}
					</h1>
					<p className="m-0 flex min-w-0 items-center gap-2.5 text-[14px] text-muted-foreground">
						<AdapterChip adapter={adapter} />
						<span className="min-w-0">
							<span className="text-foreground">{adapter}</span>
							{meta ? <> · {meta}</> : null}
						</span>
					</p>
				</div>
			</div>
			<div className="flex flex-wrap items-center gap-2.5">
				<Button
					type="button"
					variant="outline"
					className={ACTION}
					onClick={onAssign}
				>
					<Plus aria-hidden="true" />
					Assigner une tâche
				</Button>
				<Button
					type="button"
					variant="outline"
					className={ACTION}
					disabled={busy}
					onClick={onTogglePause}
				>
					{paused ? <Play aria-hidden="true" /> : <Pause aria-hidden="true" />}
					{paused ? 'Reprendre' : 'Pause'}
				</Button>
				{agent.archived ? (
					<Button
						type="button"
						variant="outline"
						className={ACTION}
						disabled={busy}
						onClick={onToggleArchive}
					>
						Restaurer
					</Button>
				) : (
					<DropdownMenu>
						<DropdownMenuTrigger asChild>
							<Button
								type="button"
								variant="ghost"
								size="icon"
								className="size-9"
								aria-label="Plus d'actions"
								disabled={busy}
							>
								<MoreHorizontal aria-hidden="true" />
							</Button>
						</DropdownMenuTrigger>
						<DropdownMenuContent align="start">
							<DropdownMenuItem
								variant="destructive"
								onSelect={onToggleArchive}
							>
								<Archive aria-hidden="true" />
								Archiver
							</DropdownMenuItem>
						</DropdownMenuContent>
					</DropdownMenu>
				)}
			</div>
		</header>
	)
}
