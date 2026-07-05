// SOURCE: github.com/JuliusBrussee/compact README (fetched 2026-07-06)
use super::CompactError;
use super::preserve::preserved_spans;

pub(super) fn check_lossless(original: &str, compressed: &str) -> Result<(), CompactError> {
    for span in preserved_spans(original) {
        if !compressed.contains(span.as_str()) {
            return Err(CompactError::PreservedTokenDropped(span));
        }
    }
    Ok(())
}
