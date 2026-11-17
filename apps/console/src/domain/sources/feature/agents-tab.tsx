import { Link } from '@tanstack/react-router'
import { useState } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import { TextInput } from '#/components/ui/text-input'
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
		<div className="flex flex-col gap-4">
			<SectionCard title="Agents à l'écoute" flush>
				{listening.length > 0 ? (
					<ul className="divide-y">
						{listening.map((agent) => (
							<li key={agent.id} className="flex items-center gap-3 px-3 py-2">
								<AgentAvatar agent={agent} size={28} />
								<span className="min-w-0 flex-1">
									<b className="font-medium">
										<Link
											to="/agents/$agentId/{-$tab}"
											params={{ agentId: agent.id, tab: 'sources' }}
											className="hover:underline"
										>
											{agent.name}
										</Link>
									</b>
									<span className="block text-muted-foreground">
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
					<p className="p-3 text-muted-foreground">
						Aucun agent n'écoute cette source. Les événements sont journalisés
						mais aucune tâche n'est créée.
					</p>
				)}
			</SectionCard>
			{free.length > 0 && choice ? (
				<SectionCard title="Ajouter un agent">
					<div className="flex gap-2">
						<NativeSelect
							aria-label="Agent"
							value={choice}
							onChange={(e) => setPicked(e.target.value)}
						>
							{free.map((a) => (
								<option key={a.id} value={a.id}>
									{a.name}
								</option>
							))}
						</NativeSelect>
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
					</div>
				</SectionCard>
			) : null}
			<SectionCard title="Tâche créée à chaque événement">
				<form
					className="flex flex-col gap-2"
					onSubmit={(e) => {
						e.preventDefault()
						updateSource(source.id, { template })
						notify('Modèle enregistré')
					}}
				>
					<div className="flex gap-2">
						<TextInput
							aria-label="Modèle de tâche"
							value={template}
							onChange={(e) => setTemplate(e.target.value)}
						/>
						<Button type="submit" variant="outline">
							Enregistrer
						</Button>
					</div>
					<p className="text-muted-foreground">
						Variable disponible :{' '}
						<span className="font-mono">{typeInfo(source.type).variable}</span>
					</p>
				</form>
			</SectionCard>
		</div>
	)
}
