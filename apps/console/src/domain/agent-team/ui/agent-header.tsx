import { Play } from 'lucide-react'
import { Button } from '#/components/ui/button'
import { AdapterChip } from '#/domain/agent-team/ui/adapter-chip'
import { AgentAvatar } from '#/domain/agent-team/ui/agent-avatar'
import type { DemoAgent } from '#/domain/demo'

interface Props {
	agent: DemoAgent
	adapter: string
	title: string
	paused: boolean
	onMakePrimary: () => void
	onAssign: () => void
	onRun: () => void
	onTogglePause: () => void
}

export function AgentHeader({
	agent,
	adapter,
	title,
	paused,
	onMakePrimary,
	onAssign,
	onRun,
	onTogglePause,
}: Props) {
	return (
		<header className="mb-3 flex flex-wrap items-end gap-x-3.5 gap-y-2.5">
			<AgentAvatar hue={agent.hue} size={40} />
			<div className="min-w-0">
				<h1 className="m-0 text-xl leading-[1.2] font-semibold tracking-[-0.01em]">
					{agent.name}
				</h1>
				<span className="flex items-center gap-1.5 text-xs text-muted-foreground">
					<AdapterChip adapter={adapter} />
					{adapter} · {title}
				</span>
			</div>
			<div className="ml-auto flex flex-wrap items-center gap-2">
				<Button type="button" variant="outline" onClick={onMakePrimary}>
					Définir comme principal
				</Button>
				<Button type="button" variant="outline" onClick={onAssign}>
					Assigner une tâche
				</Button>
				<Button type="button" variant="outline" onClick={onRun}>
					<Play aria-hidden="true" />
					Lancer
				</Button>
				<Button type="button" variant="outline" onClick={onTogglePause}>
					{paused ? 'Reprendre' : 'Pause'}
				</Button>
			</div>
		</header>
	)
}
