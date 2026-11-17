import { Link } from '@tanstack/react-router'
import type { ReactNode } from 'react'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import { NativeSelect } from '#/components/ui/native-select'
import { TextInput } from '#/components/ui/text-input'
import type { DemoAgent, DemoSourceType } from '#/domain/demo'
import {
	DEDUPE_OPTIONS,
	exampleTitle,
	MODE_OPTIONS,
	SOURCE_TYPES,
	type SourceForm,
	typeInfo,
} from '#/domain/sources/source-form'
import { SourceTypeTile } from '#/domain/sources/ui/source-type-tile'

interface Props {
	form: SourceForm
	agents: ReadonlyArray<DemoAgent>
	error: string | null
	onChange: (patch: Partial<SourceForm>) => void
	onType: (type: DemoSourceType) => void
	onSubmit: () => void
}

function Row({
	label,
	hint,
	children,
}: {
	label: string
	hint?: string
	children: ReactNode
}) {
	return (
		<div className="flex flex-col gap-1 px-3 py-2 sm:flex-row sm:items-center sm:gap-4">
			<span className="w-44 shrink-0 font-medium">{label}</span>
			<div className="flex min-w-0 flex-1 flex-col gap-1">
				{children}
				{hint ? (
					<span className="text-xs text-muted-foreground">{hint}</span>
				) : null}
			</div>
		</div>
	)
}

export function SourceFormView({
	form,
	agents,
	error,
	onChange,
	onType,
	onSubmit,
}: Props) {
	const info = typeInfo(form.type)
	const options = (values: ReadonlyArray<string>) =>
		values.map((v) => (
			<option key={v} value={v}>
				{v}
			</option>
		))
	return (
		<form
			className="flex flex-col gap-4"
			onSubmit={(e) => {
				e.preventDefault()
				onSubmit()
			}}
		>
			<SectionCard title="1. Type de source">
				<div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
					{SOURCE_TYPES.map((t) => (
						<button
							key={t.id}
							type="button"
							aria-pressed={form.type === t.id}
							onClick={() => onType(t.id)}
							className="flex items-start gap-2 rounded-md border p-2 text-left aria-pressed:border-primary aria-pressed:bg-accent"
						>
							<SourceTypeTile type={t.id} />
							<span className="min-w-0">
								<b className="block font-medium">{t.label}</b>
								<span className="text-muted-foreground">{t.description}</span>
							</span>
						</button>
					))}
				</div>
			</SectionCard>
			<SectionCard title="2. Ce qui déclenche" flush>
				<div className="divide-y">
					<Row label="Nom">
						<TextInput
							aria-label="Nom"
							placeholder="Nouveaux sous-domaines sensibles"
							value={form.name}
							onChange={(e) => onChange({ name: e.target.value })}
						/>
					</Row>
					<Row
						label={info.patternsLabel}
						hint="Sépare par des virgules. * remplace n'importe quels caractères."
					>
						<TextInput
							className="font-mono"
							aria-label={info.patternsLabel}
							placeholder={info.placeholder}
							value={form.patterns}
							onChange={(e) => onChange({ patterns: e.target.value })}
						/>
					</Row>
					<Row label="Exclusions" hint="Ces valeurs ne créent jamais de tâche">
						<TextInput
							className="font-mono"
							aria-label="Exclusions"
							placeholder="*.getnatalia.com"
							value={form.exclusions}
							onChange={(e) => onChange({ exclusions: e.target.value })}
						/>
					</Row>
					<Row label="Données">
						<NativeSelect
							aria-label="Données"
							value={form.feed}
							onChange={(e) => onChange({ feed: e.target.value })}
						>
							{options(info.feeds)}
						</NativeSelect>
					</Row>
					<Row label="Dédoublonnage" hint="Ignore une valeur déjà vue">
						<NativeSelect
							aria-label="Dédoublonnage"
							value={form.dedupe}
							onChange={(e) => onChange({ dedupe: e.target.value })}
						>
							{options(DEDUPE_OPTIONS)}
						</NativeSelect>
					</Row>
				</div>
			</SectionCard>
			<SectionCard title="3. Qui reçoit le travail" flush>
				<div className="divide-y">
					<Row
						label="Agent"
						hint="Il se réveille dès qu'un événement correspond"
					>
						<NativeSelect
							aria-label="Agent"
							value={form.agentId}
							onChange={(e) => onChange({ agentId: e.target.value })}
						>
							{agents.map((a) => (
								<option key={a.id} value={a.id}>
									{a.name}
								</option>
							))}
						</NativeSelect>
					</Row>
					<Row label="Tâches">
						<NativeSelect
							aria-label="Tâches"
							value={form.mode}
							onChange={(e) => onChange({ mode: e.target.value })}
						>
							{options(MODE_OPTIONS)}
						</NativeSelect>
					</Row>
					<Row label="Titre de la tâche" hint={`Variable : ${info.variable}`}>
						<TextInput
							aria-label="Titre de la tâche"
							value={form.template}
							onChange={(e) => onChange({ template: e.target.value })}
						/>
					</Row>
					<p className="px-3 py-2 text-muted-foreground">
						Exemple : <span className="font-mono">{exampleTitle(form)}</span>
					</p>
				</div>
			</SectionCard>
			{error ? (
				<p role="alert" className="text-signal-danger">
					{error}
				</p>
			) : null}
			<div className="flex justify-end gap-2">
				<Button asChild variant="outline">
					<Link to="/sources">Annuler</Link>
				</Button>
				<Button type="submit">Créer la source</Button>
			</div>
		</form>
	)
}
