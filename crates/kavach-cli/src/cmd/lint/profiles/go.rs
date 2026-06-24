// Canonical strict Go profile — high-signal golangci-lint enable set on top of
// the defaults. SOURCE: https://golangci-lint.run/docs/linters/ (confirmed 2026-06-24).

/// A strict `.golangci.yml` body `kavach lint init` writes for a Go project.
pub(crate) const GO_GOLANGCI: &str = r"linters:
  enable:
    - errcheck
    - govet
    - staticcheck
    - ineffassign
    - unused
    - gosec
    - revive
    - gocritic
    - errorlint
    - bodyclose
";
