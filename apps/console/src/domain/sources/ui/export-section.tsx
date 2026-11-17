import type { ReactNode } from 'react'
import { NativeSelect } from '#/components/ui/native-select'
import { Switch } from '#/components/ui/switch'

interface Props {
	destination: string
	action: string
	dedupeKey: string
	dry: boolean
	destinations: ReadonlyArray<string>
	actions: ReadonlyArray<string>
	keys: ReadonlyArray<string>
	message: string
	payload: Record<string, string> | null
	onDestination: (value: string) => void
	onAction: (value: string) => void
	onKey: (value: string) => void
	onDry: (value: boolean) => void
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
		<div className="flex flex-wrap items-center gap-x-4 gap-y-1 px-3 py-2">
			<span className="w-44 shrink-0 font-medium">{label}</span>
			{children}
			{hint ? <span className="text-muted-foreground">{hint}</span> : null}
		</div>
	)
}

export function ExportSection({
	destination,
	action,
	dedupeKey,
	dry,
	destinations,
	actions,
	keys,
	message,
	payload,
	onDestination,
	onAction,
	onKey,
	onDry,
}: Props) {
	const options = (values: ReadonlyArray<string>) =>
		values.map((v) => (
			<option key={v} value={v}>
				{v}
			</option>
		))
	return (
		<div className="divide-y">
			<Row label="Destination">
				<NativeSelect
					aria-label="Destination"
					value={destination}
					onChange={(e) => onDestination(e.target.value)}
				>
					{options(destinations)}
				</NativeSelect>
			</Row>
			<Row label="Action">
				<NativeSelect
					aria-label="Action"
					value={action}
					onChange={(e) => onAction(e.target.value)}
				>
					{options(actions)}
				</NativeSelect>
			</Row>
			<Row
				label="Clé anti-doublon"
				hint="Si la fiche existe, on ajoute une note"
			>
				<NativeSelect
					aria-label="Clé anti-doublon"
					value={dedupeKey}
					onChange={(e) => onKey(e.target.value)}
				>
					{options(keys)}
				</NativeSelect>
			</Row>
			<Row
				label="Essai à blanc"
				hint="Affiche ce qui serait écrit, sans écrire"
			>
				<Switch
					aria-label="Essai à blanc"
					checked={dry}
					onCheckedChange={onDry}
				/>
			</Row>
			<div className="flex flex-col gap-1 px-3 py-2">
				<p className="text-muted-foreground">{message}</p>
				{payload ? (
					<pre className="overflow-auto rounded-md border bg-background p-2 font-mono text-xs">
						{JSON.stringify(payload, null, 1)}
					</pre>
				) : null}
			</div>
		</div>
	)
}
