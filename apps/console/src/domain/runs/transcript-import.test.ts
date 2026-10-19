import { describe, expect, it } from 'vitest'
import {
	fileExtension,
	importTranscript,
	isAudioFile,
	MAX_AUDIO_BYTES,
	MAX_FILE_BYTES,
	titleFromFileName,
	validateTranscriptFile,
} from '#/domain/runs/transcript-import'

const VTT = `WEBVTT

NOTE
Généré automatiquement
par l'outil de visio

1
00:00:01.000 --> 00:00:03.500
<v Alice>On livre vendredi.</v>

2
00:00:03.500 --> 00:00:05.000
<v Alice>Je préviens le client.</v>

intro
00:00:05.000 --> 00:00:07.000
<v Bob Martin>D'accord,
je m'occupe du devis.</v>

00:00:07.000 --> 00:00:09.000
<v Alice>Merci &amp; à lundi.</v>
`

const SRT = `1
00:00:01,000 --> 00:00:03,500
Alice: On livre vendredi.

2
00:00:03,500 --> 00:00:05,000
<i>Bob:</i> Je prépare le devis
et je l'envoie.

3
00:00:05,000 --> 00:00:06,000
Sans locuteur.
`

describe('importTranscript', () => {
	it('turns a vtt into speaker lines and detects the participants', () => {
		const result = importTranscript('reunion.vtt', VTT)

		expect(result.text).toBe(
			[
				'Alice : On livre vendredi. Je préviens le client.',
				"Bob Martin : D'accord, je m'occupe du devis.",
				'Alice : Merci & à lundi.',
			].join('\n'),
		)
		expect(result.participants).toEqual(['Alice', 'Bob Martin'])
	})

	it('drops the header, NOTE blocks, cue numbers and timestamps', () => {
		const { text } = importTranscript('a.vtt', VTT)

		expect(text).not.toMatch(/WEBVTT|NOTE|-->|Généré|^\d+$/m)
	})

	it('handles crlf line endings and a byte order mark', () => {
		const result = importTranscript('a.vtt', `﻿${VTT.replace(/\n/g, '\r\n')}`)

		expect(result.participants).toEqual(['Alice', 'Bob Martin'])
	})

	it('keeps a vtt cue without a voice tag as plain text', () => {
		const result = importTranscript(
			'a.vtt',
			'WEBVTT\n\n00:00:01.000 --> 00:00:02.000\nBonjour <c>à tous</c>\n',
		)

		expect(result.text).toBe('Bonjour à tous')
		expect(result.participants).toEqual([])
	})

	it('splits several voices in one cue', () => {
		const result = importTranscript(
			'a.vtt',
			'WEBVTT\n\n00:00:01.000 --> 00:00:02.000\n<v A>Oui.</v><v B>Non.</v>\n',
		)

		expect(result.text).toBe('A : Oui.\nB : Non.')
	})

	it('strips srt numbers and timestamps and keeps the Name: prefix', () => {
		const result = importTranscript('a.srt', SRT)

		expect(result.text).toBe(
			[
				'Alice: On livre vendredi.',
				"Bob: Je prépare le devis et je l'envoie.",
				'Sans locuteur.',
			].join('\n'),
		)
		expect(result.participants).toEqual([])
	})

	it('keeps md and txt untouched', () => {
		const markdown = '# Réunion\n\n- Alice : on livre\n\n\n00:01 --> 00:02\n'

		expect(importTranscript('a.md', markdown).text).toBe(markdown)
		expect(importTranscript('a.TXT', markdown)).toEqual({
			text: markdown,
			participants: [],
		})
	})
})

describe('file helpers', () => {
	it('derives the title from the file name', () => {
		expect(titleFromFileName('Comité du lundi.vtt')).toBe('Comité du lundi')
		expect(titleFromFileName('notes.v2.md')).toBe('notes.v2')
		expect(titleFromFileName('README')).toBe('README')
	})

	it('reads the extension in lower case', () => {
		expect(fileExtension('A.VTT')).toBe('.vtt')
		expect(fileExtension('none')).toBe('')
	})

	it('accepts the four text formats up to 5 Mo', () => {
		for (const name of ['a.txt', 'a.md', 'a.vtt', 'a.SRT']) {
			expect(validateTranscriptFile({ name, size: MAX_FILE_BYTES })).toBe(
				undefined,
			)
		}
	})

	it('refuses other formats and larger files', () => {
		expect(validateTranscriptFile({ name: 'a.pdf', size: 10 })).toMatch(
			/Format non pris en charge/,
		)
		expect(
			validateTranscriptFile({ name: 'a.txt', size: MAX_FILE_BYTES + 1 }),
		).toBe('Le fichier dépasse 5 Mo.')
	})

	it('accepts the six audio formats up to 25 Mo', () => {
		for (const name of ['a.mp3', 'a.m4a', 'a.WAV', 'a.webm', 'a.ogg', 'a.aac']) {
			expect(isAudioFile(name)).toBe(true)
			expect(validateTranscriptFile({ name, size: MAX_AUDIO_BYTES })).toBe(
				undefined,
			)
			expect(validateTranscriptFile({ name, size: MAX_AUDIO_BYTES + 1 })).toBe(
				'Le fichier dépasse 25 Mo.',
			)
		}
		expect(isAudioFile('a.vtt')).toBe(false)
	})

	it('keeps the 5 Mo limit for text even when audio is larger', () => {
		expect(
			validateTranscriptFile({ name: 'a.md', size: MAX_FILE_BYTES + 1 }),
		).toBe('Le fichier dépasse 5 Mo.')
	})
})
