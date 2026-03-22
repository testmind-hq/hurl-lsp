use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};

pub fn execution_diagnostics_for_result(line: u32, success: bool, detail: &str) -> Vec<Diagnostic> {
    if success {
        return Vec::new();
    }
    vec![Diagnostic {
        range: Range::new(Position::new(line, 0), Position::new(line, 1)),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("hurl-lsp-run".to_string()),
        message: format!("Run failed: {detail}"),
        ..Default::default()
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_error_diagnostic_for_failed_run() {
        let diagnostics =
            execution_diagnostics_for_result(12, false, "assert failed: status == 200");
        assert_eq!(diagnostics.len(), 1);
        let diag = &diagnostics[0];
        assert_eq!(diag.range.start.line, 12);
        assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
        assert!(diag.message.contains("assert failed"));
    }
}
