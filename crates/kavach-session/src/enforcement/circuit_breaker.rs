use crate::state::SessionState;

impl SessionState {
    pub fn record_gate_block(&mut self, category: &str) -> bool {
        let count = self.gate_block_counts.entry(category.into()).or_insert(0);
        *count = count.saturating_add(1);
        let prev_count = count.saturating_sub(1);
        let should_trip = *count >= self.gate_circuit_breaker_threshold;
        let pushed_trip =
            should_trip && !self.tripped_gate_categories.contains(&category.to_owned());
        if pushed_trip {
            self.tripped_gate_categories.push(category.into());
        }
        match self.save() {
            Ok(()) => should_trip,
            Err(e) => {
                tracing::warn!(
                    error = %e, category,
                    "kavach-session: gate-block persist failed — failing closed (enforce block, rolling back unpersisted in-memory trip state)"
                );
                if pushed_trip {
                    self.tripped_gate_categories.retain(|c| c != category);
                }
                if let Some(c) = self.gate_block_counts.get_mut(category) {
                    *c = prev_count;
                }
                false
            }
        }
    }

    #[must_use]
    pub fn is_gate_tripped(&self, category: &str) -> bool {
        self.tripped_gate_categories.iter().any(|c| c == category)
    }

    #[must_use]
    pub fn gate_block_count(&self, category: &str) -> i32 {
        self.gate_block_counts
            .get(category)
            .map_or(0, |count| *count)
    }
}
