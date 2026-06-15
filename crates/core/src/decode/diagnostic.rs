

use crate::error::PrismResult;
use crate::types::report::{DiagnosticReport, RootCause, SuggestedFix};
use crate::xdr::codec::XdrCodec;
use stellar_xdr::curr::{DiagnosticEvent, ContractEventBody, ScVal};

pub fn enrich_report(
    report: &mut DiagnosticReport,
    tx_data: &serde_json::Value,
) -> PrismResult<()> {
    if let Some(events_b64) = tx_data.get("diagnosticEventsXdr").and_then(|e| e.as_array()) {
        for event_b64 in events_b64 {
            if let Some(b64_str) = event_b64.as_str() {
                if let Ok(event) = DiagnosticEvent::from_xdr_base64(b64_str) {
                    analyze_diagnostic_event(report, &event);
                }
            }
        }
    }

    Ok(())
}

fn scval_to_string(val: &ScVal) -> Option<String> {
    match val {
        ScVal::Symbol(sym) => Some(sym.to_string()),
        ScVal::String(s) => Some(s.to_string()),
        ScVal::U32(u) => Some(u.to_string()),
        ScVal::I32(i) => Some(i.to_string()),
        ScVal::U64(u) => Some(u.to_string()),
        ScVal::I64(i) => Some(i.to_string()),
        _ => None,
    }
}

fn analyze_diagnostic_event(report: &mut DiagnosticReport, event: &DiagnosticEvent) {
    if let ContractEventBody::V0(v0) = &event.event.body {
        let topics: Vec<String> = v0.topics.iter().filter_map(scval_to_string).collect();
        if topics.is_empty() {
            return;
        }

        if topics.iter().any(|t| t.to_lowercase().contains("budget") || t.to_lowercase().contains("limit")) {
            if !report.root_causes.iter().any(|c| c.description.contains("Resource limit")) {
                report.root_causes.push(RootCause {
                    description: "Resource limit was exceeded during contract execution.".to_string(),
                    likelihood: "high".to_string(),
                });
            }
            if !report.suggested_fixes.iter().any(|f| f.id == "increase_limits") {
                report.suggested_fixes.push(SuggestedFix {
                    description: "Increase the resource limits when simulating/submitting the transaction.".to_string(),
                    difficulty: "easy".to_string(),
                    requires_upgrade: false,
                    example: None,
                    id: "increase_limits".to_string(),
                    remedy_code: None,
                });
            }
        }

        if topics.iter().any(|t| t.to_lowercase().contains("storage") || t.to_lowercase().contains("footprint")) {
            if !report.root_causes.iter().any(|c| c.description.contains("footprint")) {
                report.root_causes.push(RootCause {
                    description: "The contract accessed or requested a storage key that was not declared in the footprint.".to_string(),
                    likelihood: "high".to_string(),
                });
            }
            if !report.suggested_fixes.iter().any(|f| f.id == "resimulate_footprint") {
                report.suggested_fixes.push(SuggestedFix {
                    description: "Re-simulate the transaction to capture the correct footprint keys and footprint declaration.".to_string(),
                    difficulty: "easy".to_string(),
                    requires_upgrade: false,
                    example: None,
                    id: "resimulate_footprint".to_string(),
                    remedy_code: None,
                });
            }
        }

        if topics.iter().any(|t| t.to_lowercase().contains("auth") || t.to_lowercase().contains("signature")) {
            if !report.root_causes.iter().any(|c| c.description.contains("authorization")) {
                report.root_causes.push(RootCause {
                    description: "Transaction verification or authorization check failed in __check_auth or signature check.".to_string(),
                    likelihood: "high".to_string(),
                });
            }
            if !report.suggested_fixes.iter().any(|f| f.id == "check_auth_signatures") {
                report.suggested_fixes.push(SuggestedFix {
                    description: "Check that the transaction signatures match the required signers and the nonce is correct.".to_string(),
                    difficulty: "medium".to_string(),
                    requires_upgrade: false,
                    example: None,
                    id: "check_auth_signatures".to_string(),
                    remedy_code: None,
                });
            }
        }

        let topics_str = topics.join(" > ");
        if !report.detailed_explanation.contains(&topics_str) {
            if report.detailed_explanation.is_empty() {
                report.detailed_explanation = format!("Diagnostic events trace:\n- [{}]", topics_str);
            } else {
                report.detailed_explanation.push_str(&format!("\n- [{}]", topics_str));
            }
        }
    }
}
