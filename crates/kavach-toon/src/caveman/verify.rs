// SOURCE: github.com/JuliusBrussee/caveman README (fetched 2026-07-06)
use super::CavemanError;
use super::preserve::preserved_spans;

pub(super) fn check_lossless(original: &str, compressed: &str) -> Result<(), CavemanError> {
    for span in preserved_spans(original) {
        if !compressed.contains(span.as_str()) {
            return Err(CavemanError::PreservedTokenDropped(span));
        }
    }
    Ok(())
}
