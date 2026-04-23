use std::fmt;

/// Severity level for a diagnostic finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorLevel {
    Ok,
    Warn,
    Fail,
}

impl DoctorLevel {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Fail => "FAIL",
        }
    }
}

/// Single diagnostic finding in the doctor report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorFinding {
    pub level: DoctorLevel,
    pub name: String,
    pub detail: String,
}

/// Human-readable aggregate report for environment diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub findings: Vec<DoctorFinding>,
}

impl fmt::Display for DoctorReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for finding in &self.findings {
            writeln!(
                formatter,
                "[{}] {}: {}",
                finding.level.label(),
                finding.name,
                finding.detail
            )?;
        }
        Ok(())
    }
}
