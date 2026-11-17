import { Plus } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { useNotice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { DEMO_PROJECTS } from '#/domain/demo'
import { ProjectCard } from '#/domain/projects/ui/project-card'
import { Toast } from '#/domain/tasks/ui/toast'

export function ProjectsPage() {
	const [message, notify] = useNotice()
	return (
		<Page className="leading-[1.4]">
			<DemoBanner />
			<PageHeader
				title="Projets"
				description={`${DEMO_PROJECTS.length} projets actifs`}
				actions={
					<Button onClick={() => notify('Création de projet')}>
						<Plus aria-hidden="true" />
						Nouveau projet
					</Button>
				}
			/>
			<div className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-2.5">
				{DEMO_PROJECTS.map((project) => (
					<ProjectCard key={project.name} project={project} />
				))}
			</div>
			<Toast message={message} />
		</Page>
	)
}
