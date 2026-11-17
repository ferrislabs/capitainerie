import { type DemoProject, demoAgent, shortName } from '#/domain/demo'
import { AgentChip } from '#/domain/tasks/ui/agent-chip'

export function ProjectCard({ project }: { project: DemoProject }) {
	const agent = demoAgent(project.agentId)
	return (
		<article className="flex min-w-0 flex-col gap-2 rounded-lg border bg-card px-3 py-[11px]">
			<h3 className="m-0 text-[13px] font-semibold">{project.name}</h3>
			<p className="m-0 text-muted-foreground">{project.description}</p>
			<div
				role="progressbar"
				aria-label={`Avancement de ${project.name}`}
				aria-valuenow={project.progress}
				aria-valuemin={0}
				aria-valuemax={100}
				className="h-1.5 min-w-[60px] overflow-hidden rounded-[3px] bg-secondary"
			>
				<div
					className="h-full rounded-[3px] bg-primary"
					style={{ width: `${project.progress}%` }}
				/>
			</div>
			<div className="flex items-center justify-between gap-2">
				<AgentChip
					name={agent ? shortName(agent.name) : project.agentId}
					hue={agent?.hue ?? 250}
				/>
				<span className="text-muted-foreground tabular-nums">
					{project.progress} % · {project.openTasks} ouvertes
				</span>
			</div>
		</article>
	)
}
