//! Error classification and user-friendly guidance for delivery failures

use serde::{Deserialize, Serialize};

/// Classification categories for delivery errors
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    AuthInvalid,          // 401
    AuthMissing,          // 403
    EndpointGone,         // 404
    RateLimited,          // 429
    TargetError,          // 500-599
    Unreachable,          // Connection refused/reset
    Timeout,              // Request timeout
    AuthNotConfigured,    // Empty auth header
    Unknown,              // Unclassifiable
}

/// Structured diagnosis of a delivery failure with user-friendly guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDiagnosis {
    pub category: ErrorCategory,
    pub user_message: String,
    pub guidance: String,
    pub risk_summary: Option<String>,
}

/// Diagnose a delivery error and provide user-friendly guidance
///
/// # Arguments
/// * `status_code` - HTTP status code if available
/// * `error_text` - Raw error message from delivery attempt
/// * `source_name` - Name of the source that failed
/// * `endpoint_name` - Name of the target endpoint
pub fn diagnose_error(
    status_code: Option<u16>,
    error_text: &str,
    source_name: &str,
    endpoint_name: &str,
) -> ErrorDiagnosis {
    // Try to classify by HTTP status code first
    if let Some(code) = status_code {
        match code {
            401 => ErrorDiagnosis {
                category: ErrorCategory::AuthInvalid,
                user_message: format!("Authentication rejected by {}", endpoint_name),
                guidance: format!(
                    "Check your API key in Settings > Targets > {}. The current key may have expired or been revoked.",
                    endpoint_name
                ),
                risk_summary: Some(format!(
                    "Your {} data is not reaching {}.",
                    source_name, endpoint_name
                )),
            },
            403 => ErrorDiagnosis {
                category: ErrorCategory::AuthMissing,
                user_message: format!("Not authorized to reach {}", endpoint_name),
                guidance: "This webhook requires authentication. Go to the binding settings and add an API key or auth header.".to_string(),
                risk_summary: Some(format!(
                    "Your {} data is not reaching {}.",
                    source_name, endpoint_name
                )),
            },
            404 => ErrorDiagnosis {
                category: ErrorCategory::EndpointGone,
                user_message: format!("{} no longer exists", endpoint_name),
                guidance: format!(
                    "The webhook URL may have changed. Check your target configuration and update the endpoint in Sources > {}.",
                    source_name
                ),
                risk_summary: Some(format!(
                    "Your {} data is being discarded.",
                    source_name
                )),
            },
            429 => ErrorDiagnosis {
                category: ErrorCategory::RateLimited,
                user_message: "Target is rate-limiting requests".to_string(),
                guidance: format!(
                    "Too many requests to {}. LocalPush will retry with backoff. No action needed unless this persists.",
                    endpoint_name
                ),
                risk_summary: None, // Temporary condition, auto-retries
            },
            500..=599 => ErrorDiagnosis {
                category: ErrorCategory::TargetError,
                user_message: format!("{} had an internal error", endpoint_name),
                guidance: format!(
                    "The problem is on {}'s side. LocalPush will retry automatically. If it persists, check {}'s logs.",
                    endpoint_name, endpoint_name
                ),
                risk_summary: None, // Target's problem, auto-retries
            },
            _ => classify_by_text(error_text, source_name, endpoint_name),
        }
    } else {
        classify_by_text(error_text, source_name, endpoint_name)
    }
}

/// Classify error by text patterns when HTTP status is unavailable
fn classify_by_text(error_text: &str, source_name: &str, endpoint_name: &str) -> ErrorDiagnosis {
    let lower = error_text.to_lowercase();

    if lower.contains("connection refused") || lower.contains("connection reset") {
        ErrorDiagnosis {
            category: ErrorCategory::Unreachable,
            user_message: format!("Can't reach {}", endpoint_name),
            guidance: format!(
                "Is {} running? Check the URL in Settings > Targets. LocalPush will keep retrying.",
                endpoint_name
            ),
            risk_summary: Some(format!(
                "Your {} data is queued but cannot be delivered.",
                source_name
            )),
        }
    } else if lower.contains("timeout") || lower.contains("timed out") {
        ErrorDiagnosis {
            category: ErrorCategory::Timeout,
            user_message: format!("{} didn't respond in time", endpoint_name),
            guidance: "The request took too long. This could be a network issue or the target is overloaded. Will retry.".to_string(),
            risk_summary: None, // Temporary condition, auto-retries
        }
    } else if lower.contains("authorization") && lower.contains("empty") {
        ErrorDiagnosis {
            category: ErrorCategory::AuthNotConfigured,
            user_message: "Authentication not set up for this binding".to_string(),
            guidance: "You configured an Authorization header but didn't save a credential. Open the binding config and enter your API key.".to_string(),
            risk_summary: Some(format!(
                "Your {} data is not reaching {} until authentication is configured.",
                source_name, endpoint_name
            )),
        }
    } else {
        ErrorDiagnosis {
            category: ErrorCategory::Unknown,
            user_message: format!("Delivery to {} failed", endpoint_name),
            guidance: format!(
                "Unexpected error: {}. Check your network connection and target configuration.",
                error_text
            ),
            risk_summary: Some(format!(
                "Your {} data is not reaching {}.",
                source_name, endpoint_name
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnose_401() {
        let diagnosis = diagnose_error(Some(401), "Unauthorized", "Claude Stats", "Metrick KPI");
        assert_eq!(diagnosis.category, ErrorCategory::AuthInvalid);
        assert!(diagnosis.user_message.contains("Authentication rejected"));
        assert!(diagnosis.guidance.contains("API key"));
    }

    #[test]
    fn test_diagnose_403() {
        let diagnosis = diagnose_error(Some(403), "Forbidden", "Claude Stats", "Metrick KPI");
        assert_eq!(diagnosis.category, ErrorCategory::AuthMissing);
        assert!(diagnosis.user_message.contains("Not authorized"));
        assert!(diagnosis.guidance.contains("authentication"));
    }

    #[test]
    fn test_diagnose_404() {
        let diagnosis = diagnose_error(Some(404), "Not Found", "Claude Stats", "Metrick KPI");
        assert_eq!(diagnosis.category, ErrorCategory::EndpointGone);
        assert!(diagnosis.user_message.contains("no longer exists"));
        assert!(diagnosis.guidance.contains("webhook URL"));
    }

    #[test]
    fn test_diagnose_429() {
        let diagnosis = diagnose_error(Some(429), "Too Many Requests", "Claude Stats", "Metrick KPI");
        assert_eq!(diagnosis.category, ErrorCategory::RateLimited);
        assert!(diagnosis.user_message.contains("rate-limiting"));
        assert!(diagnosis.risk_summary.is_none()); // Temporary, auto-retries
    }

    #[test]
    fn test_diagnose_500() {
        let diagnosis = diagnose_error(Some(500), "Internal Server Error", "Claude Stats", "Metrick KPI");
        assert_eq!(diagnosis.category, ErrorCategory::TargetError);
        assert!(diagnosis.user_message.contains("internal error"));
        assert!(diagnosis.guidance.contains("LocalPush will retry"));
    }

    #[test]
    fn test_diagnose_connection_refused() {
        let diagnosis = diagnose_error(None, "Connection refused", "Claude Stats", "Metrick KPI");
        assert_eq!(diagnosis.category, ErrorCategory::Unreachable);
        assert!(diagnosis.user_message.contains("Can't reach"));
        assert!(diagnosis.guidance.contains("Is"));
    }

    #[test]
    fn test_diagnose_timeout() {
        let diagnosis = diagnose_error(None, "Request timed out", "Claude Stats", "Metrick KPI");
        assert_eq!(diagnosis.category, ErrorCategory::Timeout);
        assert!(diagnosis.user_message.contains("didn't respond"));
        assert!(diagnosis.guidance.contains("network"));
    }

    #[test]
    fn test_diagnose_empty_auth() {
        let diagnosis = diagnose_error(None, "Authorization header is empty", "Claude Stats", "Metrick KPI");
        assert_eq!(diagnosis.category, ErrorCategory::AuthNotConfigured);
        assert!(diagnosis.user_message.contains("Authentication not set up"));
        assert!(diagnosis.guidance.contains("binding config"));
    }

    #[test]
    fn test_diagnose_unknown() {
        let diagnosis = diagnose_error(None, "Some weird error", "Claude Stats", "Metrick KPI");
        assert_eq!(diagnosis.category, ErrorCategory::Unknown);
        assert!(diagnosis.user_message.contains("failed"));
        assert!(diagnosis.guidance.contains("Unexpected error"));
    }

    #[test]
    fn test_diagnosis_includes_source_and_endpoint() {
        let diagnosis = diagnose_error(Some(401), "Unauthorized", "My Source", "My Endpoint");
        assert!(diagnosis.user_message.contains("My Endpoint"));
        assert!(diagnosis.risk_summary.unwrap().contains("My Source"));
    }
}
