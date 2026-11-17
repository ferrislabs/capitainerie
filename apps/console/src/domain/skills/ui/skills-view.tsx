import { Plus } from 'lucide-react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Notice } from '#/components/layout/notice'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { Button } from '#/components/ui/button'
import { HumanAvatar } from '#/domain/audit/ui/avatar'
import { Panel, Row, Rows, RowTitle } from '#/domain/audit/ui/rows'
import { Sev } from '#/domain/audit/ui/sev'
import type { DemoSkill } from '#/domain/skills/mock/skills'
import { SkillToggle } from '#/domain/skills/ui/skill-toggle'

interface Props {
	skills: ReadonlyArray<DemoSkill>
	notice: string | null
	onNewFolder: () => void
	onCreate: () => void
}

export function SkillsView({ skills, notice, onNewFolder, onCreate }: Props) {
	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Compétences"
				description="88 compétences disponibles pour l'organisation"
				actions={
					<>
						<Button variant="outline" onClick={onNewFolder}>
							Nouveau dossier
						</Button>
						<Button onClick={onCreate}>
							<Plus aria-hidden="true" />
							Nouvelle
						</Button>
					</>
				}
			/>
			<Notice message={notice} />
			<Panel>
				<Rows>
					{skills.map((s) => (
						<Row key={s.name}>
							<HumanAvatar
								initials={s.name.charAt(0).toUpperCase()}
								size={26}
							/>
							<RowTitle title={s.name} sub={s.description} />
							<span className="text-xs whitespace-nowrap text-muted-foreground">
								{s.agents} agents
							</span>
							<Sev tone="info">{s.scope}</Sev>
							<SkillToggle label={`Activer ${s.name}`} />
						</Row>
					))}
				</Rows>
			</Panel>
		</Page>
	)
}
