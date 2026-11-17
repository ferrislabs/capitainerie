import { Play } from 'lucide-react'
import { PageHeader } from '#/components/layout/page-header'
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
		<PageHeader
			leading={<AgentAvatar name={agent.name} hue={agent.hue} size={40} />}
			title={agent.name}
			description={
				<span className="inline-flex items-center gap-2">
					<AdapterChip adapter={adapter} />
					{adapter} · {title}
				</span>
			}
			actions={
				<>
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
						{paused ? 'Relancer' : 'Pause'}
					</Button>
				</>
			}
		/>
	)
}
