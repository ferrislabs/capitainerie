import { useState } from 'react'
import { useNotice } from '#/components/layout/notice'
import { DEMO_SKILLS } from '#/domain/skills/mock/skills'
import { filterSkills } from '#/domain/skills/search'
import { SkillsView } from '#/domain/skills/ui/skills-view'

export function SkillsPage() {
	const [query, setQuery] = useState('')
	const [notice, setNotice] = useNotice()
	return (
		<SkillsView
			skills={filterSkills(DEMO_SKILLS, query)}
			query={query}
			notice={notice}
			onQueryChange={setQuery}
			onCreate={() => setNotice('Importer ou écrire une compétence')}
		/>
	)
}
