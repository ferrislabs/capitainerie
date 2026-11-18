import { useMemo, useState } from 'react'
import { useNotice } from '#/components/layout/notice'
import { useTeamAgents } from '#/domain/agent-team/hooks/use-team-agents'
import {
	useArchiveSkill,
	useCreateSkill,
	useSkills,
	useUpdateSkill,
} from '#/domain/skills/hooks/use-skills'
import {
	agentsUsing,
	createBody,
	describeSkillError,
	draftOf,
	EMPTY_DRAFT,
	filterSkills,
	type Skill,
	type SkillDraft,
	updateBody,
} from '#/domain/skills/model'
import { SkillDetailSheet } from '#/domain/skills/ui/skill-detail-sheet'
import { SkillFormDialog } from '#/domain/skills/ui/skill-form-dialog'
import { SkillsView } from '#/domain/skills/ui/skills-view'

type Editing = { mode: 'create' } | { mode: 'edit'; skill: Skill } | null

export function SkillsPage() {
	const [showArchived, setShowArchived] = useState(false)
	const [query, setQuery] = useState('')
	const [selectedId, setSelectedId] = useState<string | null>(null)
	const [editing, setEditing] = useState<Editing>(null)
	const [formError, setFormError] = useState<string | null>(null)
	const [notice, setNotice] = useNotice()
	const skills = useSkills(showArchived)
	const agents = useTeamAgents(false).data?.items
	const create = useCreateSkill()
	const update = useUpdateSkill()
	const archive = useArchiveSkill()

	const items = skills.data?.items ?? []
	const visible = useMemo(() => filterSkills(items, query), [items, query])
	const selected = items.find((s) => s.id === selectedId) ?? null
	const users = selected ? agentsUsing(agents ?? [], selected.slug) : []

	const closeForm = () => {
		setEditing(null)
		setFormError(null)
	}

	const save = (draft: SkillDraft) => {
		setFormError(null)
		const onError = (error: unknown) => setFormError(describeSkillError(error))
		if (editing?.mode === 'edit') {
			const body = updateBody(editing.skill, draft)
			if (Object.keys(body).length === 0) {
				closeForm()
				return
			}
			update.mutate(
				{ path: { skill_id: editing.skill.id }, body },
				{
					onSuccess: (saved) => {
						closeForm()
						setNotice(`Compétence « ${saved.name} » enregistrée`)
					},
					onError,
				},
			)
			return
		}
		create.mutate(
			{ body: createBody(draft) },
			{
				onSuccess: (created) => {
					closeForm()
					setNotice(`Compétence « ${created.name} » créée`)
				},
				onError,
			},
		)
	}

	const setArchived = (skill: Skill, archived: boolean) => {
		const done = {
			onSuccess: () => {
				setSelectedId(null)
				setNotice(
					archived
						? `Compétence « ${skill.name} » archivée`
						: `Compétence « ${skill.name} » restaurée`,
				)
			},
			onError: (error: unknown) => setNotice(describeSkillError(error)),
		}
		if (archived) archive.mutate({ path: { skill_id: skill.id } }, done)
		else
			update.mutate(
				{ path: { skill_id: skill.id }, body: { archived: false } },
				done,
			)
	}

	return (
		<>
			<SkillsView
				skills={visible}
				total={items.length}
				loading={skills.isPending}
				error={skills.isError}
				query={query}
				showArchived={showArchived}
				notice={notice}
				onQueryChange={setQuery}
				onToggleArchived={() => {
					setSelectedId(null)
					setShowArchived((value) => !value)
				}}
				onRetry={() => void skills.refetch()}
				onCreate={() => setEditing({ mode: 'create' })}
				onOpen={(skill) => setSelectedId(skill.id)}
			/>
			<SkillDetailSheet
				skill={editing ? null : selected}
				agents={users}
				busy={archive.isPending || update.isPending}
				onClose={() => setSelectedId(null)}
				onEdit={(skill) => setEditing({ mode: 'edit', skill })}
				onArchive={setArchived}
			/>
			<SkillFormDialog
				open={editing !== null}
				mode={editing?.mode ?? 'create'}
				initial={
					editing?.mode === 'edit' ? draftOf(editing.skill) : EMPTY_DRAFT
				}
				pending={create.isPending || update.isPending}
				error={formError}
				onSubmit={save}
				onClose={closeForm}
			/>
		</>
	)
}
