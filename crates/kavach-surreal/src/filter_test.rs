use crate::filter::{FilterBuilder, FilterExpr, FilterValue};

#[test]
fn test_eq_filter() {
    let filter = FilterExpr::Eq {
        field: "entry_status".to_owned(),
        value: FilterValue::String("verified".to_owned()),
    };
    assert_eq!(filter.to_surql(), "entry_status = 'verified'");
}

#[test]
fn test_in_filter() {
    let filter = FilterExpr::In {
        field: "category".to_owned(),
        values: vec![
            FilterValue::String("arch".to_owned()),
            FilterValue::String("spec".to_owned()),
        ],
    };
    assert_eq!(filter.to_surql(), "category IN ['arch', 'spec']");
}

#[test]
fn test_range_filter() {
    let filter = FilterExpr::Range {
        field: "created_at".to_owned(),
        gte: Some(FilterValue::RelativeDuration("30d".to_owned())),
        lte: None,
    };
    assert_eq!(filter.to_surql(), "created_at >= time::now() - 30d");
}

#[test]
fn test_and_filter() {
    let filter = FilterExpr::And(vec![
        FilterExpr::Eq {
            field: "entry_status".to_owned(),
            value: FilterValue::String("verified".to_owned()),
        },
        FilterExpr::In {
            field: "category".to_owned(),
            values: vec![FilterValue::String("arch".to_owned())],
        },
    ]);
    assert_eq!(
        filter.to_surql(),
        "(entry_status = 'verified' AND category IN ['arch'])"
    );
}

#[test]
fn test_related_to_filter() {
    let filter = FilterExpr::RelatedTo {
        edge: "serves".to_owned(),
        target_table: "roadmap".to_owned(),
        target_key: "payment-flow".to_owned(),
    };
    assert_eq!(
        filter.to_surql(),
        "->serves->(roadmap WHERE entry_key = 'payment-flow')"
    );
}

#[test]
fn test_builder() {
    let Some(filter) = FilterBuilder::new()
        .eq("entry_status", "verified")
        .in_set("category", ["arch", "spec"])
        .since("created_at", "30d")
        .build()
    else {
        panic!("builder returned None for non-empty expressions");
    };

    let surql = filter.to_surql();
    assert!(surql.contains("entry_status = 'verified'"));
    assert!(surql.contains("category IN ['arch', 'spec']"));
    assert!(surql.contains("created_at >= time::now() - 30d"));
}

#[test]
fn injection_field_name_fails_closed() {
    let evil = FilterExpr::Eq {
        field: "1=1 OR title".to_owned(),
        value: FilterValue::String("x".to_owned()),
    };
    assert_eq!(evil.to_surql(), "1 = 2");
    let ok = FilterExpr::Eq {
        field: "entry_status".to_owned(),
        value: FilterValue::String("verified".to_owned()),
    };
    assert_eq!(ok.to_surql(), "entry_status = 'verified'");
}

#[test]
fn injection_edge_table_fails_closed() {
    let evil = FilterExpr::RelatedTo {
        edge: "serves; DELETE roadmap".to_owned(),
        target_table: "roadmap".to_owned(),
        target_key: "k".to_owned(),
    };
    assert_eq!(evil.to_surql(), "1 = 2");
    let evil_tbl = FilterExpr::RelatedTo {
        edge: "serves".to_owned(),
        target_table: "roadmap WHERE 1=1".to_owned(),
        target_key: "k".to_owned(),
    };
    assert_eq!(evil_tbl.to_surql(), "1 = 2");
}

#[test]
fn injection_duration_fails_closed() {
    let evil = FilterExpr::Range {
        field: "created_at".to_owned(),
        gte: Some(FilterValue::RelativeDuration("30d; DROP".to_owned())),
        lte: None,
    };
    assert!(evil.to_surql().contains("1970-01-01"));
    assert!(!evil.to_surql().contains("DROP"));
}

#[test]
fn test_string_escape() {
    let filter = FilterExpr::Eq {
        field: "title".to_owned(),
        value: FilterValue::String("it's a test".to_owned()),
    };
    assert_eq!(filter.to_surql(), "title = 'it\\'s a test'");
}
