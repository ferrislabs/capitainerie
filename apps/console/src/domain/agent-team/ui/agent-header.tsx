import { Button } from '#/components/ui/button'
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
	return (
		<header className="mb-3 flex flex-wrap items-end gap-x-3.5 gap-y-2.5">
			<Mascot
				name={agent.name}
				hue={agent.hue}
				state={mood}
				className="-mb-1 h-[60px] w-[71px] shrink-0"
			/>
			<div className="min-w-0 pb-0.5">
				<h1 className="m-0 text-xl leading-[1.2] font-semibold tracking-[-0.01em]">
					{agent.name}
					{agent.archived ? (
						<span className="ml-2 align-middle text-xs font-normal text-muted-foreground">
							Archivé
						</span>
					) : null}
				</h1>
				<span className="flex items-center gap-1.5 text-xs text-muted-foreground">
					<AdapterChip adapter={adapter} />
					{adapter} · {title}
				</span>
			</div>
			<div className="ml-auto flex flex-wrap items-center gap-2">
				<Button type="button" variant="outline" onClick={onAssign}>
					Assigner une tâche
				</Button>
				<Button
					type="button"
					variant="outline"
					disabled={busy}
					onClick={onTogglePause}
				>
					{paused ? 'Reprendre' : 'Pause'}
				</Button>
				<Button
					type="button"
					variant="outline"
					disabled={busy}
					onClick={onToggleArchive}
				>
					{agent.archived ? 'Restaurer' : 'Archiver'}
				</Button>
			</div>
		</header>
	)
}
