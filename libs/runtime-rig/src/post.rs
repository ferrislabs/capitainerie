use std::collections::HashSet;

use capitainerie_core::domain::agent::ExtractedAction;

const STOP_WORDS: &[&str] = &[
    "le", "la", "les", "l", "un", "une", "des", "de", "du", "d", "au", "aux", "en", "et", "ou",
    "a", "pour", "par", "sur", "dans", "avec", "que", "qui", "se", "s", "ce", "cette", "ces",
];

const TITLE_OVERLAP: f64 = 0.6;
const UNASSIGNED_OVERLAP: f64 = 0.5;

const DEADLINES: &[&str] = &[
    "la fin de la semaine",
    "fin de la semaine",
    "la semaine prochaine",
    "dans la semaine",
    "cette semaine",
    "cet après-midi",
    "demain matin",
    "demain soir",
    "aujourd'hui",
    "ce matin",
    "ce soir",
    "demain",
    "lundi",
    "mardi",
    "mercredi",
    "jeudi",
    "vendredi",
    "samedi",
    "dimanche",
];

pub(crate) fn clean(transcript: &str, actions: Vec<ExtractedAction>) -> Vec<ExtractedAction> {
    let prepared = actions
        .into_iter()
        .map(|mut action| {
            action.quote = strip_speaker(transcript, &action.quote);
            action.quote = repair_quote(transcript, &action.quote);
            if let Some(addressee) = addressee(transcript, &action.quote) {
                action.owner = Some(addressee);
            }
            if action.due.is_none() {
                action.due = deadline_in(&action.quote);
            }
            action
        })
        .collect();
    dedupe(prepared)
}

fn speakers(transcript: &str) -> Vec<&str> {
    transcript
        .lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(label, _)| label.trim())
        .filter(|label| {
            (1..=3).contains(&label.split_whitespace().count())
                && label.chars().next().is_some_and(char::is_uppercase)
        })
        .collect()
}

fn addressee(transcript: &str, quote: &str) -> Option<String> {
    let quote = quote.trim();
    speakers(transcript).into_iter().find_map(|speaker| {
        let rest = quote.strip_prefix(speaker)?.strip_prefix(',')?;
        let rest = rest.trim_start().to_lowercase();
        (rest.starts_with("tu ") || rest.starts_with("vous ")).then(|| speaker.to_owned())
    })
}

fn deadline_in(quote: &str) -> Option<String> {
    let lowered = quote.replace(['\u{2019}', '\u{2018}'], "'").to_lowercase();
    DEADLINES
        .iter()
        .filter_map(|phrase| lowered.find(phrase).map(|at| (at, phrase)))
        .min_by_key(|(at, phrase)| (*at, std::cmp::Reverse(phrase.len())))
        .map(|(_, phrase)| (*phrase).to_owned())
}

struct Token<'a> {
    start: usize,
    end: usize,
    word: &'a str,
}

fn tokens(text: &str) -> Vec<Token<'_>> {
    let mut found = Vec::new();
    let mut begin: Option<usize> = None;
    for (index, c) in text.char_indices() {
        if c.is_alphanumeric() {
            begin.get_or_insert(index);
        } else if let Some(start) = begin.take() {
            found.push(Token {
                start,
                end: index,
                word: &text[start..index],
            });
        }
    }
    if let Some(start) = begin {
        found.push(Token {
            start,
            end: text.len(),
            word: &text[start..],
        });
    }
    found
}

fn close(left: &str, right: &str) -> bool {
    let (left, right): (Vec<char>, Vec<char>) = (
        left.to_lowercase().chars().collect(),
        right.to_lowercase().chars().collect(),
    );
    if left == right {
        return true;
    }
    if left.len().min(right.len()) < 5 || left.len().abs_diff(right.len()) > 1 {
        return false;
    }
    let common_prefix = left.iter().zip(&right).take_while(|(a, b)| a == b).count();
    let common_suffix = left
        .iter()
        .rev()
        .zip(right.iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    common_prefix + common_suffix + 1 >= left.len().max(right.len())
}

fn contains_quote(transcript: &str, quote: &str) -> bool {
    let fold = |text: &str| {
        text.replace(['\u{2019}', '\u{2018}'], "'")
            .to_lowercase()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let needle = fold(quote);
    !needle.is_empty() && fold(transcript).contains(&needle)
}

fn repair_quote(transcript: &str, quote: &str) -> String {
    if quote.trim().is_empty() || contains_quote(transcript, quote) {
        return quote.to_owned();
    }
    let wanted = tokens(quote);
    if wanted.len() < 2 {
        return quote.to_owned();
    }
    let mut best: Option<(usize, usize, usize)> = None;
    let mut offset = 0;
    for line in transcript.split_inclusive('\n') {
        let available = tokens(line);
        let mut cursor = 0;
        let (mut hits, mut first, mut last) = (0, None, 0);
        for want in &wanted {
            let Some(found) = available[cursor..]
                .iter()
                .position(|token| close(token.word, want.word))
            else {
                continue;
            };
            let index = cursor + found;
            first.get_or_insert(index);
            last = index;
            cursor = index + 1;
            hits += 1;
        }
        if let Some(first) = first
            && best.is_none_or(|(known, _, _)| hits > known)
        {
            best = Some((
                hits,
                offset + available[first].start,
                offset + available[last].end,
            ));
        }
        offset += line.len();
    }
    match best {
        Some((hits, start, end)) if hits * 10 >= wanted.len() * 7 && hits >= 2 => {
            transcript[start..end].to_owned()
        }
        _ => quote.to_owned(),
    }
}

fn strip_speaker(transcript: &str, quote: &str) -> String {
    let trimmed = quote.trim();
    let Some((label, rest)) = trimmed.split_once(':') else {
        return quote.to_owned();
    };
    let label = label.trim();
    let rest = rest.trim();
    let words = label.split_whitespace().count();
    let starts_upper = label.chars().next().is_some_and(char::is_uppercase);
    if rest.is_empty() || !(1..=3).contains(&words) || !starts_upper {
        return quote.to_owned();
    }
    let speaks = transcript.lines().any(|line| {
        line.trim_start()
            .strip_prefix(label)
            .is_some_and(|after| after.trim_start().starts_with(':'))
    });
    if speaks {
        rest.to_owned()
    } else {
        quote.to_owned()
    }
}

fn words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect()
}

fn normalized(text: &str) -> String {
    words(text).join(" ")
}

fn content_words(text: &str) -> HashSet<String> {
    words(text)
        .into_iter()
        .filter(|word| !STOP_WORDS.contains(&word.as_str()))
        .collect()
}

fn overlap(left: &str, right: &str) -> f64 {
    let (left, right) = (content_words(left), content_words(right));
    let smallest = left.len().min(right.len());
    if smallest == 0 {
        return 0.0;
    }
    left.intersection(&right).count() as f64 / smallest as f64
}

fn same_action(left: &ExtractedAction, right: &ExtractedAction) -> bool {
    let (a, b) = (normalized(&left.quote), normalized(&right.quote));
    if a.is_empty() || b.is_empty() {
        return false;
    }
    if a == b {
        return true;
    }
    let titles = overlap(&left.title, &right.title);
    if (a.contains(&b) || b.contains(&a)) && titles >= TITLE_OVERLAP {
        return true;
    }
    let one_unassigned = left.owner.is_some() != right.owner.is_some();
    one_unassigned && shared(&left.title, &right.title) >= 2 && titles >= UNASSIGNED_OVERLAP
}

fn shared(left: &str, right: &str) -> usize {
    content_words(left)
        .intersection(&content_words(right))
        .count()
}

fn specificity(action: &ExtractedAction) -> usize {
    usize::from(action.owner.is_some()) * 2
        + usize::from(action.due.is_some()) * 2
        + usize::from(!action.description.trim().is_empty())
}

fn dedupe(actions: Vec<ExtractedAction>) -> Vec<ExtractedAction> {
    let mut kept: Vec<ExtractedAction> = Vec::with_capacity(actions.len());
    for action in actions {
        let Some(slot) = kept.iter().position(|known| same_action(known, &action)) else {
            kept.push(action);
            continue;
        };
        let known = &mut kept[slot];
        let (mut best, other) = if specificity(&action) > specificity(known) {
            (action, known.clone())
        } else {
            (known.clone(), action)
        };
        if best.owner.is_none() {
            best.owner = other.owner;
        }
        if best.due.is_none() {
            best.due = other.due;
        }
        if best.client_ref.is_none() {
            best.client_ref = other.client_ref;
        }
        if best.project_ref.is_none() {
            best.project_ref = other.project_ref;
        }
        *known = best;
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRANSCRIPT: &str = "Anne : Il faut relancer Acme.\nPaul : Je m'en occupe avant mardi.\nMarie Dupont : Note : ok.";

    fn action(title: &str, quote: &str, owner: Option<&str>, due: Option<&str>) -> ExtractedAction {
        ExtractedAction {
            title: title.into(),
            description: String::new(),
            quote: quote.into(),
            owner: owner.map(Into::into),
            due: due.map(Into::into),
            client_ref: None,
            project_ref: None,
        }
    }

    #[test]
    fn a_speaker_prefix_of_the_transcript_is_stripped() {
        assert_eq!(
            strip_speaker(TRANSCRIPT, "Paul : Je m'en occupe"),
            "Je m'en occupe"
        );
        assert_eq!(strip_speaker(TRANSCRIPT, "Marie Dupont : ok"), "ok");
        assert_eq!(
            strip_speaker(TRANSCRIPT, "Paul: Je m'en occupe"),
            "Je m'en occupe"
        );
    }

    #[test]
    fn a_colon_that_is_not_a_speaker_is_kept() {
        assert_eq!(strip_speaker(TRANSCRIPT, "Note : ok"), "Note : ok");
        assert_eq!(strip_speaker(TRANSCRIPT, "relancer Acme"), "relancer Acme");
        assert_eq!(strip_speaker(TRANSCRIPT, "Paul :"), "Paul :");
    }

    #[test]
    fn identical_quotes_are_one_action_and_the_most_specific_wins() {
        let kept = dedupe(vec![
            action("Relancer Acme", "relancer Acme", None, None),
            action(
                "Relancer Acme",
                "Relancer  acme.",
                Some("Anne"),
                Some("mardi"),
            ),
        ]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].owner.as_deref(), Some("Anne"));
        assert_eq!(kept[0].due.as_deref(), Some("mardi"));
    }

    #[test]
    fn nested_quotes_with_similar_titles_merge_and_fill_missing_fields() {
        let kept = dedupe(vec![
            action(
                "Ouvrir un ticket",
                "ouvrir un ticket prioritaire",
                Some("Lina"),
                None,
            ),
            action(
                "Ouvrir un ticket prioritaire",
                "ouvrir un ticket",
                None,
                Some("aujourd'hui"),
            ),
        ]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].owner.as_deref(), Some("Lina"));
        assert_eq!(kept[0].due.as_deref(), Some("aujourd'hui"));
    }

    #[test]
    fn an_unassigned_request_is_absorbed_by_the_action_that_took_it() {
        let kept = dedupe(vec![
            action(
                "Prévenir les clients par email",
                "prévenir les clients par email",
                None,
                None,
            ),
            action(
                "Rédiger l'email aux clients",
                "je rédige l'email aux clients",
                Some("Lina"),
                Some("lundi"),
            ),
        ]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].owner.as_deref(), Some("Lina"));
        assert_eq!(kept[0].due.as_deref(), Some("lundi"));
    }

    #[test]
    fn an_unassigned_and_an_assigned_action_on_different_objects_stay_apart() {
        let kept = dedupe(vec![
            action("Envoyer le devis", "envoyer le devis", None, None),
            action(
                "Envoyer la facture",
                "j'envoie la facture",
                Some("Paul"),
                None,
            ),
        ]);
        assert_eq!(kept.len(), 2);
    }

    #[test]
    fn the_person_a_request_is_addressed_to_owns_it() {
        let transcript = "Sophie : Lina, tu peux ouvrir un ticket ?\nLina : Oui.";
        let cleaned = clean(
            transcript,
            vec![action(
                "Ouvrir un ticket",
                "Lina, tu peux ouvrir un ticket",
                Some("Sophie"),
                None,
            )],
        );
        assert_eq!(cleaned[0].owner.as_deref(), Some("Lina"));
        assert_eq!(addressee(transcript, "Lina a ouvert un ticket"), None);
        assert_eq!(addressee(transcript, "Sophie, il faut ouvrir"), None);
    }

    #[test]
    fn a_missing_deadline_is_read_from_the_quote() {
        assert_eq!(
            deadline_in("je la livre vendredi").as_deref(),
            Some("vendredi")
        );
        assert_eq!(
            deadline_in("Je m’en occupe demain matin").as_deref(),
            Some("demain matin")
        );
        assert_eq!(
            deadline_in("je le fais aujourd'hui").as_deref(),
            Some("aujourd'hui")
        );
        assert_eq!(deadline_in("je le fais"), None);
        let cleaned = clean(
            "Paul : Je livre la maquette jeudi.",
            vec![action(
                "Livrer la maquette",
                "Paul : Je livre la maquette jeudi.",
                Some("Paul"),
                None,
            )],
        );
        assert_eq!(cleaned[0].due.as_deref(), Some("jeudi"));
        assert_eq!(cleaned[0].quote, "Je livre la maquette jeudi.");
    }

    #[test]
    fn a_slightly_mangled_quote_is_snapped_back_to_the_transcript() {
        let transcript =
            "Anne : Bonjour.\nOlivier : Le visuel, je le valide ce soir avec la direction.\n";
        assert_eq!(
            repair_quote(
                transcript,
                "Le visuel, je le validate ce soir avec la direction."
            ),
            "Le visuel, je le valide ce soir avec la direction"
        );
        assert_eq!(
            repair_quote(transcript, "je le valide ce soir"),
            "je le valide ce soir"
        );
        assert_eq!(
            repair_quote(transcript, "réserver la salle de réunion"),
            "réserver la salle de réunion"
        );
    }

    #[test]
    fn distinct_actions_are_kept_in_order() {
        let kept = dedupe(vec![
            action("Envoyer le devis", "envoyer le devis", None, None),
            action("Envoyer la facture", "envoyer la facture", None, None),
        ]);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].title, "Envoyer le devis");
    }

    #[test]
    fn nested_quotes_with_unrelated_titles_are_kept() {
        let kept = dedupe(vec![
            action(
                "Réserver la salle",
                "réserver la salle et envoyer le lien",
                None,
                None,
            ),
            action("Envoyer le lien", "envoyer le lien", None, None),
        ]);
        assert_eq!(kept.len(), 2);
    }
}
