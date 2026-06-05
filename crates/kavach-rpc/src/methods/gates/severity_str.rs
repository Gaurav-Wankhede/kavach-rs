//! Wire-format severity strings for guard enums whose own crates do not (yet)
//! expose an `as_str`. Kept as a leaf so the `gates` RPC hub stays focused on the
//! handler logic. Destructive severity/category strings are NOT here — those live
//! on the enums themselves (`DestructiveSeverity::as_str` / `DestructiveCategory::as_str`).
use kavach_patterns::{
    axum_guard::AxumSeverity, database_ops_guard::DbOpsSeverity, dsa_guard::DsaSeverity,
    finops_guard::FinopsSeverity, migration_safety_guard::MigSeverity,
    observability_guard::ObsSeverity, pii_data_guard::PiiSeverity, solid_guard::SolidSeverity,
    webhook_signature_guard::WhSeverity,
};

pub(super) const fn db_ops(s: DbOpsSeverity) -> &'static str {
    match s {
        DbOpsSeverity::P0Block => "P0Block",
        DbOpsSeverity::P1Advisory => "P1Advisory",
        DbOpsSeverity::P2Warning => "P2Warning",
    }
}

pub(super) const fn pii(s: PiiSeverity) -> &'static str {
    match s {
        PiiSeverity::P0Block => "P0Block",
        PiiSeverity::P1Advisory => "P1Advisory",
    }
}

pub(super) const fn mig(s: MigSeverity) -> &'static str {
    match s {
        MigSeverity::P0Block => "P0Block",
        MigSeverity::P1Advisory => "P1Advisory",
    }
}

pub(super) const fn wh(s: WhSeverity) -> &'static str {
    match s {
        WhSeverity::P0Block => "P0Block",
        WhSeverity::P1Advisory => "P1Advisory",
    }
}

pub(super) const fn obs(s: ObsSeverity) -> &'static str {
    match s {
        ObsSeverity::P1Advisory => "P1Advisory",
        ObsSeverity::P2Warning => "P2Warning",
    }
}

pub(super) const fn finops(s: FinopsSeverity) -> &'static str {
    match s {
        FinopsSeverity::P1Advisory => "P1Advisory",
        FinopsSeverity::P2Warning => "P2Warning",
    }
}

pub(super) const fn solid(s: SolidSeverity) -> &'static str {
    match s {
        SolidSeverity::P1Advisory => "P1Advisory",
        SolidSeverity::P2Warning => "P2Warning",
    }
}

pub(super) const fn dsa(s: DsaSeverity) -> &'static str {
    match s {
        DsaSeverity::P1Advisory => "P1Advisory",
        DsaSeverity::P2Warning => "P2Warning",
    }
}

pub(super) const fn axum(s: AxumSeverity) -> &'static str {
    match s {
        AxumSeverity::P0Block => "P0Block",
        AxumSeverity::P1Advisory => "P1Advisory",
        AxumSeverity::P2Warning => "P2Warning",
    }
}
