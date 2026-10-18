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

pub(crate) fn price_of(kind: ProviderKind, model: &str) -> Option<Price> {
    match kind {
        ProviderKind::Anthropic => CLAUDE
            .iter()
            .find(|(id, _)| *id == model)
            .map(|(_, price)| *price),
        ProviderKind::Mistral | ProviderKind::OpenAiCompatible => None,
    }
}

/// Every Claude model with its price, for pricing the usage of an execution.
pub fn claude_prices() -> Vec<(String, Pricing)> {
    CLAUDE
        .iter()
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
    fn claude_prices_cover_every_priced_model() {
        let prices = claude_prices();
        assert_eq!(prices.len(), CLAUDE.len());
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
