use capitainerie_core::domain::{provider::ProviderKind, run::step::Pricing};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Price {
    pub input_per_mtok_micros: u64,
    pub output_per_mtok_micros: u64,
}

const CLAUDE: &[(&str, Price)] = &[
    (
        "claude-opus-5-5",
        Price {
            input_per_mtok_micros: 4_000_000,
            output_per_mtok_micros: 20_000_000,
        },
    ),
    (
        "claude-sonnet-5-5",
        Price {
            input_per_mtok_micros: 2_000_000,
            output_per_mtok_micros: 10_000_000,
        },
    ),
    (
        "claude-haiku-5-5",
        Price {
            input_per_mtok_micros: 100_000,
            output_per_mtok_micros: 500_000,
        },
    ),
    (
        "claude-fable-5-1",
        Price {
            input_per_mtok_micros: 10_000_000,
            output_per_mtok_micros: 50_000_000,
        },
    ),
];

const fn price(input_per_mtok_micros: u64, output_per_mtok_micros: u64) -> Price {
    Price {
        input_per_mtok_micros,
        output_per_mtok_micros,
    }
}

const OPENAI: &[(&str, Price)] = &[
    ("gpt-6.1-sol", price(2_000_000, 10_000_000)),
    ("gpt-6.1-terra", price(1_000_000, 5_000_000)),
    ("gpt-6-astra", price(10_000_000, 50_000_000)),
    ("gpt-6-sol", price(2_000_000, 10_000_000)),
    ("gpt-6-luna", price(100_000, 500_000)),
    ("gpt-5.6-sol", price(4_000_000, 20_000_000)),
    ("gpt-5.6-terra", price(2_000_000, 12_000_000)),
    ("gpt-5.6-luna", price(200_000, 1_200_000)),
    ("gpt-5.5", price(5_000_000, 30_000_000)),
];

const GOOGLE: &[(&str, Price)] = &[
    ("gemini-4-argon", price(4_000_000, 20_000_000)),
    ("gemini-3.8-flash", price(1_500_000, 7_500_000)),
    ("gemini-3.6-flash", price(1_500_000, 7_500_000)),
    ("gemini-3.1-pro", price(2_000_000, 12_000_000)),
];

fn table(kind: ProviderKind) -> &'static [(&'static str, Price)] {
    match kind {
        ProviderKind::Anthropic => CLAUDE,
        ProviderKind::OpenAi => OPENAI,
        ProviderKind::Google => GOOGLE,
        ProviderKind::Mistral | ProviderKind::OpenAiCompatible => &[],
    }
}

pub(crate) fn price_of(kind: ProviderKind, model: &str) -> Option<Price> {
    table(kind)
        .iter()
        .find(|(id, _)| *id == model)
        .map(|(_, price)| *price)
}

/// Every model an executor runs on, with its price, for pricing the usage of
/// an execution.
pub fn executor_prices() -> Vec<(String, Pricing)> {
    CLAUDE
        .iter()
        .chain(OPENAI)
        .chain(GOOGLE)
        .map(|(model, price)| {
            (
                (*model).to_owned(),
                Pricing {
                    input_per_million_micros: price.input_per_mtok_micros,
                    output_per_million_micros: price.output_per_mtok_micros,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use capitainerie_core::domain::execution::harness::{ANTIGRAVITY_MODELS, CODEX_MODELS};

    use super::*;

    #[test]
    fn claude_prices_are_per_million_tokens_in_micros() {
        let haiku = price_of(ProviderKind::Anthropic, "claude-haiku-5-5");
        assert_eq!(
            haiku,
            Some(Price {
                input_per_mtok_micros: 100_000,
                output_per_mtok_micros: 500_000
            })
        );
        assert_eq!(
            price_of(ProviderKind::Anthropic, "claude-opus-5-5").map(|p| p.output_per_mtok_micros),
            Some(20_000_000)
        );
    }

    #[test]
    fn executor_prices_cover_every_priced_model() {
        let prices = executor_prices();
        assert_eq!(prices.len(), CLAUDE.len() + OPENAI.len() + GOOGLE.len());
        let sonnet = prices
            .iter()
            .find(|(model, _)| model == "claude-sonnet-5-5")
            .map(|(_, pricing)| *pricing);
        assert_eq!(
            sonnet,
            Some(Pricing {
                input_per_million_micros: 2_000_000,
                output_per_million_micros: 10_000_000
            })
        );
    }

    #[test]
    fn every_codex_model_has_a_price() {
        for model in CODEX_MODELS {
            assert!(price_of(ProviderKind::OpenAi, model).is_some(), "{model}");
        }
        assert_eq!(
            price_of(ProviderKind::OpenAi, "gpt-6-luna"),
            Some(price(100_000, 500_000))
        );
    }

    #[test]
    fn every_antigravity_model_has_a_price() {
        for (model, _) in ANTIGRAVITY_MODELS {
            assert!(price_of(ProviderKind::Google, model).is_some(), "{model}");
        }
    }

    #[test]
    fn an_unknown_model_or_provider_has_no_price() {
        assert_eq!(
            price_of(ProviderKind::Anthropic, "claude-haiku-5-5-20260101"),
            None
        );
        assert_eq!(
            price_of(ProviderKind::Mistral, "mistral-large-latest"),
            None
        );
        assert_eq!(
            price_of(ProviderKind::OpenAiCompatible, "claude-haiku-5-5"),
            None
        );
    }
}
