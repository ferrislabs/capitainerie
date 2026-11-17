import { Search } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { EmptyState } from '#/components/layout/empty-state'
import { Notice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { SectionCard } from '#/components/layout/section-card'
import { Badge } from '#/components/ui/badge'
import { Button } from '#/components/ui/button'
import { TextInput } from '#/components/ui/text-input'
import type { DemoSkill } from '#/domain/skills/mock/skills'

interface Props {
	skills: ReadonlyArray<DemoSkill>
	query: string
	notice: string | null
	onQueryChange: (query: string) => void
	onCreate: () => void
}

export function SkillsView({
	skills,
	query,
	notice,
	onQueryChange,
	onCreate,
}: Props) {
	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Compétences"
				description="88 compétences disponibles pour l'organisation"
				actions={<Button onClick={onCreate}>Nouvelle compétence</Button>}
			/>
			<Notice message={notice} />
			<TextInput
				type="search"
				aria-label="Rechercher une compétence"
				placeholder="Rechercher par nom"
				value={query}
				onChange={(e) => onQueryChange(e.target.value)}
			/>
			{skills.length === 0 ? (
				<EmptyState
					icon={Search}
					title="Aucune compétence"
					description="Aucune compétence ne porte ce nom."
				/>
			) : (
				<SectionCard title="Compétences" flush>
					<ul className="divide-y">
						{skills.map((s) => (
							<li key={s.name} className="flex items-center gap-3 px-3 py-2">
								<div className="flex min-w-0 flex-1 flex-col">
									<span className="font-medium">{s.name}</span>
									<span className="text-muted-foreground">{s.description}</span>
								</div>
								<span className="whitespace-nowrap text-muted-foreground">
									{s.agents} agents
								</span>
								<Badge variant="outline">{s.scope}</Badge>
							</li>
						))}
					</ul>
				</SectionCard>
			)}
		</Page>
	)
}
