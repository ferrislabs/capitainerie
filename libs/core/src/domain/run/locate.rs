/// Finds `quote` in `body` and returns its character range and the exact text
/// of the body there. A model rarely copies perfectly: spacing, case and
/// typographic quotes are forgiven, words are not. The returned text always
/// comes from the body, so the card cites what was said, not the model's copy.
pub fn locate(body: &str, quote: &str) -> Option<(u32, u32, String)> {
    let quote = quote.trim();
    if quote.is_empty() {
        return None;
    }
    let chars: Vec<char> = body.chars().collect();
    let (normalized_body, origin) = normalize(&chars);
    let (normalized_quote, _) = normalize(&quote.chars().collect::<Vec<_>>());
    let needle: Vec<char> = trim(&normalized_quote);
    if needle.is_empty() {
        return None;
    }
    let found = normalized_body
        .windows(needle.len())
        .position(|window| window == needle.as_slice())?;
    let start = origin[found];
    let end = origin[found + needle.len() - 1] + 1;
    let text: String = chars[start..end].iter().collect();
    Some((u32::try_from(start).ok()?, u32::try_from(end).ok()?, text))
}

/// Lowercased, whitespace runs collapsed to one space, typographic quotes
/// folded; `origin[i]` is the index in the input of normalized char `i`.
fn normalize(chars: &[char]) -> (Vec<char>, Vec<usize>) {
    let mut normalized = Vec::with_capacity(chars.len());
    let mut origin = Vec::with_capacity(chars.len());
    let mut previous_space = false;
    for (index, &c) in chars.iter().enumerate() {
        if c.is_whitespace() {
            if !previous_space {
                normalized.push(' ');
                origin.push(index);
            }
            previous_space = true;
            continue;
        }
        previous_space = false;
        let folded = match c {
            '\u{2018}' | '\u{2019}' | '\u{02BC}' => '\'',
            '\u{201C}' | '\u{201D}' | '\u{00AB}' | '\u{00BB}' => '"',
            '\u{2013}' | '\u{2014}' => '-',
            other => other,
        };
        for lower in folded.to_lowercase() {
            normalized.push(lower);
            origin.push(index);
        }
    }
    (normalized, origin)
}

fn trim(chars: &[char]) -> Vec<char> {
    let start = chars.iter().position(|c| *c != ' ').unwrap_or(chars.len());
    let end = chars
        .iter()
        .rposition(|c| *c != ' ')
        .map_or(start, |i| i + 1);
    chars[start..end].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = "Claire : il faut migrer la CI vers les runners ARM.\nMalik : je m’en charge,   je fais un point mardi.";

    #[test]
    fn an_exact_quote_is_found_in_characters() {
        let (start, end, text) = locate(BODY, "migrer la CI").unwrap();
        assert_eq!(text, "migrer la CI");
        assert_eq!((start, end), (17, 29));
    }

    #[test]
    fn spacing_case_and_apostrophes_are_forgiven_and_the_body_text_kept() {
        let (_, _, text) = locate(BODY, "Je m'en charge, je fais un point mardi").unwrap();
        assert_eq!(text, "je m’en charge,   je fais un point mardi");
    }

    #[test]
    fn a_paraphrase_is_not_found() {
        assert!(locate(BODY, "je vais migrer la CI").is_none());
        assert!(locate(BODY, "   ").is_none());
    }

    #[test]
    fn the_range_matches_the_returned_text() {
        let (start, end, text) = locate(BODY, "point MARDI").unwrap();
        let slice: String = BODY
            .chars()
            .skip(start as usize)
            .take((end - start) as usize)
            .collect();
        assert_eq!(slice, text);
    }
}
