import { Link } from '@tanstack/react-router'
import { Plus } from 'lucide-react'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'

export function AgentsHeader() {
	return (
		<PageHeader
			title="Agents"
			description="27 agents · 1 actif"
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
