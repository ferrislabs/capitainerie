use capitainerie_core::domain::run::step::Pricing;

const CLAUDE: &[(&str, u64, u64)] = &[
    ("claude-opus-5-5", 4_000_000, 20_000_000),
    ("claude-sonnet-5-5", 2_000_000, 10_000_000),
    ("claude-haiku-5-5", 100_000, 500_000),
    ("claude-fable-5-1", 10_000_000, 50_000_000),
];

pub fn executor_prices() -> Vec<(String, Pricing)> {
    CLAUDE
        .iter()
        .map(|(model, input, output)| {
            (
                (*model).to_owned(),
                Pricing {
                    input_per_million_micros: *input,
                    output_per_million_micros: *output,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_executor_model_has_a_price() {
        let prices = executor_prices();
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
}
