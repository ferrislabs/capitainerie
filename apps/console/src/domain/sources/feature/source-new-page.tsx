import { useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import { DemoBanner } from '#/components/layout/demo-banner'
import { Page } from '#/components/layout/page'
import { PageHeader } from '#/components/layout/page-header'
import { addSource, demoStore, useDemoState } from '#/domain/demo'
import {
	initialForm,
	type SourceForm,
	switchType,
	toSource,
	validateForm,
} from '#/domain/sources/source-form'
import { SourceFormView } from '#/domain/sources/ui/source-form-view'

const DEFAULT_AGENT = 'veille-legale'

export function SourceNewPage() {
	const agents = useDemoState((s) => s.agents)
	const navigate = useNavigate()
	const [form, setForm] = useState<SourceForm>(() =>
		initialForm(
			agents.some((a) => a.id === DEFAULT_AGENT)
				? DEFAULT_AGENT
				: (agents[0]?.id ?? ''),
		),
	)
	const [error, setError] = useState<string | null>(null)

	function submit() {
		const problem = validateForm(form)
		setError(problem)
		if (problem) return
		const source = toSource(
			form,
			demoStore.state.sources.map((s) => s.id),
		)
		addSource(source)
		void navigate({
			to: '/sources/$sourceId/{-$tab}',
			params: { sourceId: source.id },
		})
	}

	return (
		<Page>
			<DemoBanner />
			<PageHeader
				title="Nouvelle source"
				description="Choisis ce que l'agent écoute, puis qui reçoit le travail"
			/>
			<SourceFormView
				form={form}
				agents={agents}
				error={error}
				onChange={(patch) => setForm((f) => ({ ...f, ...patch }))}
				onType={(type) => setForm((f) => switchType(f, type))}
				onSubmit={submit}
			/>
		</Page>
	)
}
