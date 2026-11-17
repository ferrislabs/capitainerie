import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import {
	attachSource,
	type DemoSource,
	detachSource,
	shortName,
	updateSource,
	useDemoState,
} from '#/domain/demo'
import { typeInfo } from '#/domain/sources/source-form'
import { AgentAvatar } from '#/domain/sources/ui/agent-avatar'
import { AddRow, CTL, Empty } from '#/domain/sources/ui/mockup'
import { cn } from '#/lib/utils'

interface Props {
	source: DemoSource
	notify: (message: string) => void
}

export function AgentsTab({ source, notify }: Props) {
	const agents = useDemoState((s) => s.agents)
	const listening = source.agentIds
		.map((id) => agents.find((a) => a.id === id))
		.filter((a) => a !== undefined)
	const free = agents.filter((a) => !source.agentIds.includes(a.id))
	const [picked, setPicked] = useState('')
	const choice = free.some((a) => a.id === picked) ? picked : free[0]?.id
	const [template, setTemplate] = useState(source.template)
	return (
		<div>
			<SectionCard
				title="Agents à l'écoute"
				flush
				actions={<Link to="/agents/nouveau">Créer un agent</Link>}
			>
				{listening.length > 0 ? (
					<ul className="m-0 list-none p-0">
						{listening.map((agent) => (
							<li
								key={agent.id}
								className="flex min-h-9 w-full min-w-0 items-center gap-2.5 border-t px-3 py-1.5 first:border-t-0 hover:bg-secondary"
							>
								<AgentAvatar agent={agent} size={28} decorative />
								<span className="min-w-0 flex-1">
									<b className="block truncate font-medium">
										<Link
											to="/agents/$agentId/{-$tab}"
											params={{ agentId: agent.id, tab: 'sources' }}
											className="text-foreground"
										>
											{agent.name}
										</Link>
									</b>
									<span className="block truncate text-[12px] text-muted-foreground">
										{agent.role} · {agent.model}
									</span>
								</span>
								<Button
									type="button"
									size="sm"
									variant="outline"
									aria-label={`Retirer ${shortName(agent.name)}`}
									onClick={() => {
										detachSource(source.id, agent.id)
										notify(
											`${shortName(agent.name)} n'écoute plus ${source.name}`,
										)
									}}
								>
									Retirer
								</Button>
							</li>
						))}
					</ul>
				) : (
					<Empty title="Aucun agent n'écoute cette source">
						Les événements sont journalisés mais aucune tâche n'est créée.
					</Empty>
				)}
			</SectionCard>
			{free.length > 0 && choice ? (
				<SectionCard title="Ajouter un agent" flush>
					<AddRow className="pt-3">
						<select
							className={cn(CTL, 'flex-1')}
							aria-label="Agent"
							value={choice}
							onChange={(e) => setPicked(e.target.value)}
						>
							{free.map((a) => (
								<option key={a.id} value={a.id}>
									{a.name}
								</option>
							))}
						</select>
						<Button
							type="button"
							onClick={() => {
								const agent = agents.find((a) => a.id === choice)
								attachSource(source.id, choice)
								if (agent)
									notify(
										`${shortName(agent.name)} écoute maintenant ${source.name}`,
									)
							}}
						>
							Ajouter
						</Button>
					</AddRow>
				</SectionCard>
			) : null}
			<SectionCard title="Tâche créée à chaque événement" flush>
				<form
					onSubmit={(e) => {
						e.preventDefault()
						updateSource(source.id, { template })
						notify('Modèle enregistré')
					}}
				>
					<AddRow className="pt-3">
						<input
							className={cn(CTL, 'flex-1')}
							aria-label="Modèle de tâche"
							value={template}
							onChange={(e) => setTemplate(e.target.value)}
						/>
						<Button type="submit" variant="outline">
							Enregistrer
						</Button>
					</AddRow>
					<p className="m-0 px-3 pb-3 text-muted-foreground">
						Variable disponible :{' '}
						<span className="font-mono text-[11.5px]">
							{typeInfo(source.type).variable}
						</span>
					</p>
				</form>
			</SectionCard>
		</div>
	)
}
