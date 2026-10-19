export const ACCEPTED_EXTENSIONS = ['.txt', '.md', '.vtt', '.srt'] as const
export const MAX_FILE_BYTES = 5 * 1024 * 1024

export interface ImportedTranscript {
	text: string
	participants: string[]
}

interface Cue {
	speaker: string | null
	text: string
}

const TIMESTAMP = /-->/
const VOICE_TAG = /<v(?:\.[^\s>]*)?\s+([^>]*)>/
const ANY_TAG = /<[^>]*>/g
const SRT_MARKUP = /\{\\[^}]*\}/g

export function fileExtension(fileName: string): string {
	const dot = fileName.lastIndexOf('.')
	return dot < 0 ? '' : fileName.slice(dot).toLowerCase()
}

export function titleFromFileName(fileName: string): string {
	const dot = fileName.lastIndexOf('.')
	return (dot > 0 ? fileName.slice(0, dot) : fileName).trim()
}

export function validateTranscriptFile(file: {
	name: string
	size: number
}): string | undefined {
	const extension = fileExtension(file.name)
	if (!ACCEPTED_EXTENSIONS.some((accepted) => accepted === extension)) {
		return 'Format non pris en charge : utilisez un fichier .txt, .md, .vtt ou .srt.'
	}
	if (file.size > MAX_FILE_BYTES) {
		return 'Le fichier dépasse 5 Mo.'
	}
	return undefined
}

export function importTranscript(
	fileName: string,
	content: string,
): ImportedTranscript {
	const text = content.replace(/^﻿/, '')
	switch (fileExtension(fileName)) {
		case '.vtt':
			return fromCues(cuesOf(text, 'vtt'))
		case '.srt':
			return fromCues(cuesOf(text, 'srt'))
		default:
			return { text, participants: [] }
	}
}

function decode(value: string): string {
	return value
		.replace(/&nbsp;/g, ' ')
		.replace(/&lt;/g, '<')
		.replace(/&gt;/g, '>')
		.replace(/&amp;/g, '&')
}

function clean(value: string): string {
	return decode(value.replace(ANY_TAG, '').replace(SRT_MARKUP, ''))
		.replace(/\s+/g, ' ')
		.trim()
}

function speakerCues(raw: string): Cue[] {
	const parts = raw.split(new RegExp(VOICE_TAG.source, 'g'))
	const cues: Cue[] = [{ speaker: null, text: clean(parts[0] ?? '') }]
	for (let index = 1; index < parts.length; index += 2) {
		cues.push({
			speaker: decode(parts[index] ?? '').trim() || null,
			text: clean(parts[index + 1] ?? ''),
		})
	}
	return cues
}

function cuesOf(text: string, format: 'vtt' | 'srt'): Cue[] {
	const blocks = text
		.replace(/\r\n?/g, '\n')
		.split(/\n{2,}/)
		.map((block) => block.split('\n'))
	const cues: Cue[] = []
	for (const lines of blocks) {
		const timestamp = lines.findIndex((line) => TIMESTAMP.test(line))
		if (timestamp < 0) continue
		if (format === 'vtt' && /^(NOTE|STYLE|REGION)(\s|$)/.test(lines[0] ?? '')) {
			continue
		}
		const raw = lines.slice(timestamp + 1).join(' ')
		cues.push(...speakerCues(raw))
	}
	return cues.filter((cue) => cue.text !== '')
}

function fromCues(cues: Cue[]): ImportedTranscript {
	const merged: Cue[] = []
	for (const cue of cues) {
		const last = merged[merged.length - 1]
		if (last && cue.speaker !== null && last.speaker === cue.speaker) {
			last.text = `${last.text} ${cue.text}`
		} else {
			merged.push({ ...cue })
		}
	}
	const participants = [
		...new Set(merged.flatMap((cue) => (cue.speaker ? [cue.speaker] : []))),
	]
	const text = merged
		.map((cue) => (cue.speaker ? `${cue.speaker} : ${cue.text}` : cue.text))
		.join('\n')
	return { text, participants }
}
