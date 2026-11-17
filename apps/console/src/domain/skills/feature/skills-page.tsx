import { useNotice } from '#/components/layout/notice'
import { DEMO_SKILLS } from '#/domain/skills/mock/skills'
import { SkillsView } from '#/domain/skills/ui/skills-view'

export function SkillsPage() {
	const [notice, setNotice] = useNotice()
	return (
		<SkillsView
			skills={DEMO_SKILLS}
			notice={notice}
			onNewFolder={() => setNotice('Nouveau dossier')}
			onCreate={() => setNotice('Importer ou écrire une compétence')}
		/>
	)
}
