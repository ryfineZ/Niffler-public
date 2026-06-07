use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrometheusSample {
    pub name: String,
    pub labels: BTreeMap<String, String>,
    pub value: String,
}

pub async fn fetch_prometheus_samples(url: &str) -> Result<Vec<PrometheusSample>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|err| format!("failed to build metrics http client: {err}"))?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|err| format!("failed to fetch metrics from {url}: {err}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| format!("failed to read metrics body from {url}: {err}"))?;
    if !status.is_success() {
        return Err(format!("metrics endpoint {url} returned {status}: {body}"));
    }
    Ok(parse_prometheus_samples(&body))
}

pub fn parse_prometheus_samples(text: &str) -> Vec<PrometheusSample> {
    text.lines()
        .filter_map(parse_prometheus_line)
        .collect::<Vec<_>>()
}

pub fn find_metric_value_u64(
    samples: &[PrometheusSample],
    metric_name: &str,
    labels: &[(&str, &str)],
) -> Option<u64> {
    samples
        .iter()
        .find(|sample| {
            metric_name_matches(&sample.name, metric_name) && labels_match(sample, labels)
        })
        .and_then(|sample| sample.value.parse::<u64>().ok())
}

fn metric_name_matches(actual: &str, expected: &str) -> bool {
    actual == expected
        || actual
            .rsplit_once('_')
            .map(|(_, suffix)| suffix == expected)
            .unwrap_or(false)
        || actual.ends_with(&format!("_{expected}"))
}

fn labels_match(sample: &PrometheusSample, labels: &[(&str, &str)]) -> bool {
    labels
        .iter()
        .all(|(key, value)| sample.labels.get(*key).map(|current| current.as_str()) == Some(*value))
}

fn parse_prometheus_line(line: &str) -> Option<PrometheusSample> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (metric, value) = trimmed.rsplit_once(' ')?;
    let (name, labels) = if let Some((name, raw_labels)) = metric
        .split_once('{')
        .and_then(|(name, rest)| rest.strip_suffix('}').map(|labels| (name, labels)))
    {
        (name.to_string(), parse_labels(raw_labels))
    } else {
        (metric.to_string(), BTreeMap::new())
    };
    Some(PrometheusSample {
        name,
        labels,
        value: value.to_string(),
    })
}

fn parse_labels(raw: &str) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    for pair in raw.split(',').filter(|pair| !pair.is_empty()) {
        if let Some((key, value)) = pair.split_once('=') {
            labels.insert(
                key.trim().to_string(),
                value
                    .trim()
                    .trim_matches('"')
                    .replace("\\\"", "\"")
                    .replace("\\n", "\n")
                    .replace("\\\\", "\\"),
            );
        }
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::{find_metric_value_u64, parse_prometheus_samples};

    #[test]
    fn parses_prometheus_samples_with_labels() {
        let samples = parse_prometheus_samples(
            r#"
# HELP aether_gateway_concurrency_in_flight Current number of in-flight operations.
# TYPE aether_gateway_concurrency_in_flight gauge
aether_gateway_concurrency_in_flight{gate="gateway_requests"} 7
aether_gateway_concurrency_rejected_total{gate="gateway_requests"} 12
"#,
        );

        assert_eq!(
            find_metric_value_u64(
                &samples,
                "concurrency_in_flight",
                &[("gate", "gateway_requests")]
            ),
            Some(7)
        );
        assert_eq!(
            find_metric_value_u64(
                &samples,
                "concurrency_rejected_total",
                &[("gate", "gateway_requests")]
            ),
            Some(12)
        );
    }
}
