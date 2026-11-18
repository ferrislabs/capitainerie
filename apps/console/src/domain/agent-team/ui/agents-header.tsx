import { Link } from '@tanstack/react-router'
import { Plus } from 'lucide-react'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'

function plural(count: number, one: string, many: string) {
	return `${count} ${count > 1 ? many : one}`
}

export function AgentsHeader() {
	const query = useTeamAgents(false)
	const agents = query.data?.items ?? []
	const running = agents.filter((a) => a.status === 'running').length
	const paused = agents.filter((a) => a.paused).length
	const description = query.data
		? [
				plural(agents.length, 'agent', 'agents'),
				`${running} en cours`,
				...(paused > 0 ? [`${paused} en pause`] : []),
			].join(' · ')
		: undefined
	return (
		<PageHeader
			title="Agents"
			description={description}
			actions={
				<Button asChild>
					<Link to="/agents/nouveau">
						<Plus aria-hidden="true" />
						Nouvel agent
					</Link>
				</Button>
			}
		/>
	)
}
