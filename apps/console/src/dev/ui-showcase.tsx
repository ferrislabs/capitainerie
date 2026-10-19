import {
	Archive,
	Copy,
	Mail,
	Pencil,
	Search,
	Trash2,
	UserPlus,
} from 'lucide-react'
import { type ReactNode, useState } from 'react'
import { Button } from '#/components/ui/button'
import { Checkbox } from '#/components/ui/checkbox'
import { Combobox, Select } from '#/components/ui/combobox'
import type { ComboboxItem } from '#/components/ui/combobox-model'
import { DateInput } from '#/components/ui/date-input'
import {
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSection,
	DropdownMenuSeparator,
	DropdownMenuSub,
	DropdownMenuSubContent,
	DropdownMenuSubTrigger,
	DropdownMenuTrigger,
} from '#/components/ui/dropdown-menu'
import { Field } from '#/components/ui/field'
import { RadioGroup, RadioGroupItem } from '#/components/ui/radio-group'
import { Switch } from '#/components/ui/switch'
import { TextInput } from '#/components/ui/text-input'
import { Textarea } from '#/components/ui/textarea'

const SHIPS: ComboboxItem[] = [
	{ value: 'ancre', label: 'Ancre', description: 'Mouillage nord' },
	{ value: 'ecureuil', label: 'Écureuil' },
	{ value: 'ete', label: 'Été' },
	{ value: 'baleine', label: 'Baleine', disabled: true },
	{ value: 'cap', label: 'Cap Horn' },
]

const MODELS: ComboboxItem[] = [
	{
		label: 'Anthropic',
		options: [
			{ value: 'opus', label: 'Claude Opus' },
			{ value: 'sonnet', label: 'Claude Sonnet' },
		],
	},
	{
		label: 'OpenAI',
		options: [
			{ value: 'gpt', label: 'GPT' },
			{ value: 'mini', label: 'GPT mini' },
		],
	},
]

function Block({ title, children }: { title: string; children: ReactNode }) {
	return (
		<section className="flex flex-col gap-4">
			<h2 className="text-[15px] font-semibold">{title}</h2>
			<div className="grid gap-4 rounded-lg border bg-card p-4 sm:grid-cols-2">
				{children}
			</div>
		</section>
	)
}

function TextInputs() {
	const [name, setName] = useState('Port de Brest')
	return (
		<Block title="TextInput">
			<Field label="Nom" hint="Tel qu’il figure au contrat" required>
				<TextInput
					value={name}
					onChange={(event) => setName(event.target.value)}
					clearable
				/>
			</Field>
			<Field label="Petit format" optional>
				<TextInput size="sm" placeholder="28 px de haut" />
			</Field>
			<Field label="Recherche">
				<TextInput
					leading={<Search aria-hidden="true" />}
					placeholder="Chercher un client"
					clearable
				/>
			</Field>
			<Field label="Adresse">
				<TextInput
					leading={<Mail aria-hidden="true" />}
					trailing={<span className="text-xs">@exemple.fr</span>}
					defaultValue="contact"
				/>
			</Field>
			<Field label="En erreur" error="Ce nom est déjà pris">
				<TextInput defaultValue="Brest" />
			</Field>
			<Field label="Désactivé" disabled>
				<TextInput defaultValue="Non modifiable" />
			</Field>
			<Field label="En chargement">
				<TextInput loading defaultValue="Vérification" />
			</Field>
			<Field label="Échéance" hint="Format du navigateur">
				<DateInput
					min="2026-01-01"
					max="2026-12-31"
					defaultValue="2026-10-19"
				/>
			</Field>
			<Field label="Échéance en erreur" error="Date hors du contrat">
				<DateInput defaultValue="2027-02-01" />
			</Field>
			<Field label="Échéance désactivée" disabled>
				<DateInput defaultValue="2026-10-19" />
			</Field>
		</Block>
	)
}

function Textareas() {
	const [note, setNote] = useState('')
	return (
		<Block title="Textarea">
			<Field
				label="Note"
				count={note.length}
				max={200}
				hint="Grandit avec le texte"
			>
				<Textarea
					autoGrow
					maxLength={200}
					value={note}
					onChange={(event) => setNote(event.target.value)}
				/>
			</Field>
			<Field label="Limitée à 4 lignes" optional>
				<Textarea maxRows={4} rows={2} />
			</Field>
			<Field label="Compteur intégré">
				<Textarea showCount maxLength={80} />
			</Field>
			<Field label="En erreur" error="La description est trop vague">
				<Textarea defaultValue="Bof" />
			</Field>
		</Block>
	)
}

function Selects() {
	const [ship, setShip] = useState('')
	const [priority, setPriority] = useState('high')
	const [model, setModel] = useState('sonnet')
	const [tags, setTags] = useState<string[]>(['ancre', 'cap'])
	return (
		<Block title="Select et Combobox">
			<Field label="Select simple">
				<Select
					items={[
						{ value: 'low', label: 'Basse' },
						{ value: 'high', label: 'Haute' },
						{ value: 'urgent', label: 'Urgente' },
					]}
					value={priority}
					onValueChange={setPriority}
					placeholder="Choisir…"
				/>
			</Field>
			<Field
				label="Combobox avec « Aucun »"
				hint="Tapez pour filtrer, accents ignorés"
			>
				<Combobox
					items={SHIPS}
					value={ship}
					onValueChange={setShip}
					noneLabel="Aucun"
					placeholder="Rechercher un navire"
					clearable
				/>
			</Field>
			<Field label="Groupes">
				<Combobox
					items={MODELS}
					value={model}
					onValueChange={setModel}
					placeholder="Modèle"
				/>
			</Field>
			<Field
				label="Sélection multiple"
				hint="Retour arrière retire la dernière puce"
			>
				<Combobox
					multiple
					clearable
					items={SHIPS}
					value={tags}
					onValueChange={setTags}
					placeholder="Étiquettes"
				/>
			</Field>
			<Field label="Petit format">
				<Select
					size="sm"
					items={SHIPS}
					value=""
					onValueChange={() => {}}
					placeholder="28 px"
				/>
			</Field>
			<Field label="En erreur" error="Choisissez un navire">
				<Combobox
					items={SHIPS}
					value=""
					onValueChange={() => {}}
					placeholder="Rechercher"
				/>
			</Field>
			<Field label="En chargement">
				<Combobox
					loading
					items={[]}
					value=""
					onValueChange={() => {}}
					placeholder="Chargement des dépôts"
				/>
			</Field>
			<Field label="Liste vide" hint="Ouvrez-la pour voir le texte vide">
				<Combobox
					items={[]}
					value=""
					onValueChange={() => {}}
					emptyText="Aucun dépôt connecté"
					placeholder="Dépôt"
				/>
			</Field>
			<Field label="Désactivé" disabled>
				<Select items={SHIPS} value="ancre" onValueChange={() => {}} />
			</Field>
		</Block>
	)
}

function Menus() {
	const [done, setDone] = useState(true)
	return (
		<Block title="DropdownMenu">
			<DropdownMenu>
				<DropdownMenuTrigger asChild>
					<Button variant="outline">Actions</Button>
				</DropdownMenuTrigger>
				<DropdownMenuContent align="start" className="w-60">
					<DropdownMenuSection label="Carte">
						<DropdownMenuItem shortcut="E">
							<Pencil aria-hidden="true" />
							Modifier
						</DropdownMenuItem>
						<DropdownMenuItem shortcut="⌘D">
							<Copy aria-hidden="true" />
							Dupliquer
						</DropdownMenuItem>
						<DropdownMenuCheckboxItem
							checked={done}
							onCheckedChange={setDone}
							onSelect={(event) => event.preventDefault()}
						>
							Terminée
						</DropdownMenuCheckboxItem>
					</DropdownMenuSection>
					<DropdownMenuSeparator />
					<DropdownMenuSub>
						<DropdownMenuSubTrigger>
							<UserPlus aria-hidden="true" />
							Assigner à
						</DropdownMenuSubTrigger>
						<DropdownMenuSubContent>
							<DropdownMenuItem>Nathaël</DropdownMenuItem>
							<DropdownMenuItem>Camille</DropdownMenuItem>
						</DropdownMenuSubContent>
					</DropdownMenuSub>
					<DropdownMenuSeparator />
					<DropdownMenuSection label="Zone sensible">
						<DropdownMenuItem>
							<Archive aria-hidden="true" />
							Archiver
						</DropdownMenuItem>
						<DropdownMenuItem variant="destructive" shortcut="⌫">
							<Trash2 aria-hidden="true" />
							Supprimer
						</DropdownMenuItem>
					</DropdownMenuSection>
				</DropdownMenuContent>
			</DropdownMenu>
		</Block>
	)
}

function Choices() {
	const [priority, setPriority] = useState('high')
	const [on, setOn] = useState(true)
	return (
		<Block title="Checkbox, Switch, RadioGroup">
			<div className="flex flex-col gap-3">
				<Checkbox label="Critère rempli" description="Coché par l’équipe" />
				<Checkbox label="Coché" defaultChecked />
				<Checkbox label="Indéterminé" checked="indeterminate" />
				<Checkbox label="Désactivé" disabled />
			</div>
			<div className="flex flex-col gap-3">
				<Switch
					label="Notifications"
					description="Un courriel par carte proposée"
					checked={on}
					onCheckedChange={setOn}
				/>
				<Switch label="Désactivé" disabled />
			</div>
			<Field label="Priorité" group hint="Une seule valeur">
				<RadioGroup value={priority} onValueChange={setPriority}>
					<RadioGroupItem value="urgent" label="Urgente" />
					<RadioGroupItem
						value="high"
						label="Haute"
						description="À traiter cette semaine"
					/>
					<RadioGroupItem value="low" label="Basse" />
				</RadioGroup>
			</Field>
			<Field label="Statut" group error="Choisissez un statut">
				<RadioGroup>
					<RadioGroupItem value="a" label="Ouvert" />
					<RadioGroupItem value="b" label="Fermé" />
				</RadioGroup>
			</Field>
		</Block>
	)
}

export function UiShowcase() {
	return (
		<div className="mx-auto flex max-w-5xl flex-col gap-8 px-8 py-12">
			<header className="flex flex-col gap-1">
				<h1 className="text-2xl font-semibold tracking-tight">
					Composants de formulaire
				</h1>
				<p className="text-[13px] text-muted-foreground">
					Page de revue, visible uniquement en développement. Chaque composant
					est montré dans ses états.
				</p>
			</header>
			<TextInputs />
			<Textareas />
			<Selects />
			<Menus />
			<Choices />
		</div>
	)
}
