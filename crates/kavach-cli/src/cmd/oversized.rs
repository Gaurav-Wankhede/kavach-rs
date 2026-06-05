mod scan;

use crate::cli::OversizedAction;

pub(super) fn run(action: OversizedAction) -> i32 {
    match action {
        OversizedAction::Scan {
            dir,
            threshold,
            format,
        } => scan::run(&dir, threshold, format),
    }
}
