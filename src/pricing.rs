#[derive(Debug, Clone)]
pub struct ModelPrice {
    pub id: &'static str,
    pub label: &'static str,
    pub input_per_m: f64,
    pub output_per_m: f64,
    pub cached_input_per_m: Option<f64>,
    pub context: Option<&'static str>,
    pub note: Option<&'static str>,
}

pub const OFFICIAL_PRICES: &[ModelPrice] = &[
    ModelPrice {
        id: "grok-4.5",
        label: "Grok 4.5",
        input_per_m: 2.0,
        output_per_m: 6.0,
        cached_input_per_m: Some(0.5),
        context: Some("500k"),
        note: Some("xAI API · default in USA (elsewhere often via VPN)"),
    },
    ModelPrice {
        id: "grok-build",
        label: "grok-build",
        input_per_m: 1.0,
        output_per_m: 2.0,
        cached_input_per_m: None,
        context: Some("256k"),
        note: Some("xAI Code API · extra option in some regions (e.g. EU)"),
    },
    ModelPrice {
        id: "grok-build-0.1",
        label: "grok-build-0.1",
        input_per_m: 1.0,
        output_per_m: 2.0,
        cached_input_per_m: None,
        context: Some("256k"),
        note: Some("Official Code API id (same rates as grok-build)"),
    },
    ModelPrice {
        id: "grok-composer-2.5-fast",
        label: "Composer 2.5 Fast",
        input_per_m: 3.0,
        output_per_m: 15.0,
        cached_input_per_m: None,
        context: Some("200k"),
        note: Some("Cursor list price · default outside USA"),
    },
    ModelPrice {
        id: "grok-composer-2.5",
        label: "Composer 2.5 Standard",
        input_per_m: 0.5,
        output_per_m: 2.5,
        cached_input_per_m: None,
        context: Some("200k"),
        note: Some("Cursor list price · Standard tier"),
    },
];

pub const PRICING_SOURCE: &str = "https://docs.x.ai/developers/pricing";
pub const PRICING_UPDATED: &str = "2026-07";

#[derive(Debug, Clone)]
pub struct CostBreakdown {
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub priced: bool,
    pub price: ModelPriceOwned,
}

#[derive(Debug, Clone)]
pub struct ModelPriceOwned {
    pub id: String,
    #[allow(dead_code)]
    pub label: String,
    pub input_per_m: f64,
    pub output_per_m: f64,
    #[allow(dead_code)]
    pub cached_input_per_m: Option<f64>,
    #[allow(dead_code)]
    pub context: Option<String>,
    #[allow(dead_code)]
    pub note: Option<String>,
}

impl From<&ModelPrice> for ModelPriceOwned {
    fn from(p: &ModelPrice) -> Self {
        Self {
            id: p.id.to_string(),
            label: p.label.to_string(),
            input_per_m: p.input_per_m,
            output_per_m: p.output_per_m,
            cached_input_per_m: p.cached_input_per_m,
            context: p.context.map(|s| s.to_string()),
            note: p.note.map(|s| s.to_string()),
        }
    }
}

fn unpriced(model_id: &str) -> ModelPriceOwned {
    ModelPriceOwned {
        id: model_id.to_string(),
        label: if model_id == "unknown" {
            "Unknown".into()
        } else {
            model_id.to_string()
        },
        input_per_m: 0.0,
        output_per_m: 0.0,
        cached_input_per_m: None,
        context: None,
        note: Some("Not a known Grok Build model / no public list price".into()),
    }
}

pub fn resolve_price(model_id: Option<&str>) -> ModelPriceOwned {
    let Some(model_id) = model_id else {
        return unpriced("unknown");
    };

    let key = model_id.to_lowercase();
    let key = key.trim();

    if let Some(p) = OFFICIAL_PRICES.iter().find(|p| p.id.eq_ignore_ascii_case(key)) {
        return p.into();
    }

    if key.starts_with("grok-build") {
        return OFFICIAL_PRICES
            .iter()
            .find(|p| p.id == "grok-build")
            .unwrap()
            .into();
    }
    if key.starts_with("grok-4.5") {
        return OFFICIAL_PRICES
            .iter()
            .find(|p| p.id == "grok-4.5")
            .unwrap()
            .into();
    }
    if key.contains("composer") {
        if key.contains("standard") && !key.contains("fast") {
            return OFFICIAL_PRICES
                .iter()
                .find(|p| p.id == "grok-composer-2.5")
                .unwrap()
                .into();
        }
        return OFFICIAL_PRICES
            .iter()
            .find(|p| p.id == "grok-composer-2.5-fast")
            .unwrap()
            .into();
    }

    unpriced(model_id)
}

pub fn calc_cost(model_id: Option<&str>, input_tokens: u64, output_tokens: u64) -> CostBreakdown {
    let price = resolve_price(model_id);
    let priced = price.input_per_m > 0.0 || price.output_per_m > 0.0;
    let input_cost = (input_tokens as f64 / 1_000_000.0) * price.input_per_m;
    let output_cost = (output_tokens as f64 / 1_000_000.0) * price.output_per_m;
    CostBreakdown {
        input_cost,
        output_cost,
        total_cost: input_cost + output_cost,
        priced,
        price,
    }
}
