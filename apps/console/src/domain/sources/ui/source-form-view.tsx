import { Link } from '@tanstack/react-router'
import { SectionCard } from '#/components/layout/section-card'
import { Button } from '#/components/ui/button'
import type { DemoAgent, DemoSourceType } from '#/domain/demo'
import {
	DEDUPE_OPTIONS,
	exampleTitle,
	MODE_OPTIONS,
	SOURCE_TYPES,
	type SourceForm,
	typeInfo,
} from '#/domain/sources/source-form'
import { CTL, Fr } from '#/domain/sources/ui/mockup'
import { cn } from '#/lib/utils'

interface Props {
	form: SourceForm
	agents: ReadonlyArray<DemoAgent>
	error: string | null
	onChange: (patch: Partial<SourceForm>) => void
	onType: (type: DemoSourceType) => void
	onSubmit: () => void
}

const Row = Fr

const MONO = 'font-mono text-[11.5px]'

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
			className="mx-auto mt-[2vh] flex w-full max-w-[880px] flex-col gap-7"
			onSubmit={(e) => {
				e.preventDefault()
				onSubmit()
			}}
		>
			<SectionCard title="1. Type de source" flush>
				<div className="grid grid-cols-[repeat(auto-fill,minmax(250px,1fr))] gap-2.5 p-2.5">
					{SOURCE_TYPES.map((t) => (
						<button
							key={t.id}
							type="button"
							aria-pressed={form.type === t.id}
							onClick={() => onType(t.id)}
							className="flex cursor-pointer items-start gap-2 rounded-lg border bg-card px-2.5 py-[9px] text-left text-foreground aria-pressed:border-primary aria-pressed:bg-primary/8"
						>
							<span
								aria-hidden="true"
								className="block size-7 shrink-0 rounded-full bg-secondary text-[12px] font-semibold text-muted-foreground"
							>
								{t.id.slice(0, 3)}
							</span>
							<span className="min-w-0">
								<b className="font-semibold">
									{t.label}
									<span className="block text-[12px] text-muted-foreground">
										{t.description}
									</span>
								</b>
							</span>
						</button>
					))}
				</div>
			</SectionCard>
			<SectionCard title="2. Ce qui déclenche" flush>
				<div>
					<Row label="Nom">
						<input
							className={CTL}
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
						<input
							className={cn(CTL, MONO)}
							aria-label={info.patternsLabel}
							placeholder={info.placeholder}
							value={form.patterns}
							onChange={(e) => onChange({ patterns: e.target.value })}
						/>
					</Row>
					<Row label="Exclusions" hint="Ces valeurs ne créent jamais de tâche">
						<input
							className={cn(CTL, MONO)}
							aria-label="Exclusions"
							placeholder="*.getnatalia.com"
							value={form.exclusions}
							onChange={(e) => onChange({ exclusions: e.target.value })}
						/>
					</Row>
					<Row label="Données">
						<select
							className={CTL}
							aria-label="Données"
							value={form.feed}
							onChange={(e) => onChange({ feed: e.target.value })}
						>
							{options(info.feeds)}
						</select>
					</Row>
					<Row label="Dédoublonnage" hint="Ignore une valeur déjà vue">
						<select
							className={CTL}
							aria-label="Dédoublonnage"
							value={form.dedupe}
							onChange={(e) => onChange({ dedupe: e.target.value })}
						>
							{options(DEDUPE_OPTIONS)}
						</select>
					</Row>
				</div>
			</SectionCard>
			<SectionCard title="3. Qui reçoit le travail" flush>
				<div>
					<Row
						label="Agent"
						hint="Il se réveille dès qu'un événement correspond"
					>
						<select
							className={CTL}
							aria-label="Agent"
							value={form.agentId}
							onChange={(e) => onChange({ agentId: e.target.value })}
						>
							{agents.map((a) => (
								<option key={a.id} value={a.id}>
									{a.name}
								</option>
							))}
						</select>
					</Row>
					<Row label="Tâches">
						<select
							className={CTL}
							aria-label="Tâches"
							value={form.mode}
							onChange={(e) => onChange({ mode: e.target.value })}
						>
							{options(MODE_OPTIONS)}
						</select>
					</Row>
					<Row label="Titre de la tâche" hint={`Variable : ${info.variable}`}>
						<input
							className={CTL}
							aria-label="Titre de la tâche"
							value={form.template}
							onChange={(e) => onChange({ template: e.target.value })}
						/>
					</Row>
					<p className="m-0 px-3 pb-3 text-muted-foreground">
						Exemple :{' '}
						<span className="font-mono text-[11.5px]">
							{exampleTitle(form)}
						</span>
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
