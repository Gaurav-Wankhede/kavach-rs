use super::expr::FilterExpr;
use super::value::FilterValue;

#[derive(Debug, Default)]
pub struct FilterBuilder {
    expressions: Vec<FilterExpr>,
}

impl FilterBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn eq(mut self, field: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.expressions.push(FilterExpr::Eq {
            field: field.into(),
            value: value.into(),
        });
        self
    }

    #[must_use]
    pub fn ne(mut self, field: impl Into<String>, value: impl Into<FilterValue>) -> Self {
        self.expressions.push(FilterExpr::Ne {
            field: field.into(),
            value: value.into(),
        });
        self
    }

    #[must_use]
    pub fn in_set(
        mut self,
        field: impl Into<String>,
        values: impl IntoIterator<Item = impl Into<FilterValue>>,
    ) -> Self {
        self.expressions.push(FilterExpr::In {
            field: field.into(),
            values: values.into_iter().map(Into::into).collect(),
        });
        self
    }

    #[must_use]
    pub fn not_in_set(
        mut self,
        field: impl Into<String>,
        values: impl IntoIterator<Item = impl Into<FilterValue>>,
    ) -> Self {
        self.expressions.push(FilterExpr::NotIn {
            field: field.into(),
            values: values.into_iter().map(Into::into).collect(),
        });
        self
    }

    #[must_use]
    pub fn range(
        mut self,
        field: impl Into<String>,
        gte: Option<impl Into<FilterValue>>,
        lte: Option<impl Into<FilterValue>>,
    ) -> Self {
        self.expressions.push(FilterExpr::Range {
            field: field.into(),
            gte: gte.map(Into::into),
            lte: lte.map(Into::into),
        });
        self
    }

    #[must_use]
    pub fn since(mut self, field: impl Into<String>, duration: impl Into<String>) -> Self {
        self.expressions.push(FilterExpr::Range {
            field: field.into(),
            gte: Some(FilterValue::RelativeDuration(duration.into())),
            lte: None,
        });
        self
    }

    #[must_use]
    pub fn contains(mut self, field: impl Into<String>, substring: impl Into<String>) -> Self {
        self.expressions.push(FilterExpr::Contains {
            field: field.into(),
            substring: substring.into(),
        });
        self
    }

    #[must_use]
    pub fn related_to(
        mut self,
        edge: impl Into<String>,
        target_table: impl Into<String>,
        target_key: impl Into<String>,
    ) -> Self {
        self.expressions.push(FilterExpr::RelatedTo {
            edge: edge.into(),
            target_table: target_table.into(),
            target_key: target_key.into(),
        });
        self
    }

    #[must_use]
    pub fn build(self) -> Option<FilterExpr> {
        match self.expressions.len() {
            0 => None,
            1 => self.expressions.into_iter().next(),
            _ => Some(FilterExpr::And(self.expressions)),
        }
    }
}
