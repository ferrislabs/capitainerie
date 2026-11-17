import { CTL, Fr, Sw } from '#/domain/sources/ui/mockup'

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

const Row = Fr

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
		<div>
			<Row label="Destination">
				<select
					className={CTL}
					aria-label="Destination"
					value={destination}
					onChange={(e) => onDestination(e.target.value)}
				>
					{options(destinations)}
				</select>
			</Row>
			<Row label="Action">
				<select
					className={CTL}
					aria-label="Action"
					value={action}
					onChange={(e) => onAction(e.target.value)}
				>
					{options(actions)}
				</select>
			</Row>
			<Row
				label="Clé anti-doublon"
				hint="Si la fiche existe, on ajoute une note"
			>
				<select
					className={CTL}
					aria-label="Clé anti-doublon"
					value={dedupeKey}
					onChange={(e) => onKey(e.target.value)}
				>
					{options(keys)}
				</select>
			</Row>
			<Row
				label="Essai à blanc"
				hint="Affiche ce qui serait écrit, sans écrire"
			>
				<Sw aria-label="Essai à blanc" checked={dry} onCheckedChange={onDry} />
			</Row>
			<div className="px-3 pb-3">
				<div className="mb-1 text-muted-foreground">{message}</div>
				{payload ? (
					<pre className="m-0 overflow-auto rounded-md border bg-background p-2 font-mono text-[11px]">
						{JSON.stringify(payload, null, 1)}
					</pre>
				) : null}
			</div>
		</div>
	)
}
