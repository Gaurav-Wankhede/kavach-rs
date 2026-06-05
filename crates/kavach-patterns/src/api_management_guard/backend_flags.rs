#[derive(Clone, Copy)]
pub(super) struct BackendFlags(u8);

impl BackendFlags {
    const PAGINATION: u8 = 1 << 0;
    const OPENAPI: u8 = 1 << 1;
    const RATE_LIMIT: u8 = 1 << 2;
    const PROBLEM_DETAILS: u8 = 1 << 3;
    const IDEMPOTENCY_KEY: u8 = 1 << 4;

    pub(super) const fn builder() -> BackendFlagsBuilder {
        BackendFlagsBuilder(0)
    }

    pub(super) const fn has_pagination(self) -> bool {
        (self.0 & Self::PAGINATION) != 0
    }
    pub(super) const fn has_openapi(self) -> bool {
        (self.0 & Self::OPENAPI) != 0
    }
    pub(super) const fn has_rate_limit(self) -> bool {
        (self.0 & Self::RATE_LIMIT) != 0
    }
    pub(super) const fn has_problem_details(self) -> bool {
        (self.0 & Self::PROBLEM_DETAILS) != 0
    }
    pub(super) const fn has_idempotency_key(self) -> bool {
        (self.0 & Self::IDEMPOTENCY_KEY) != 0
    }
}

pub(super) struct BackendFlagsBuilder(u8);

impl BackendFlagsBuilder {
    pub(super) const fn pagination(mut self, val: bool) -> Self {
        if val {
            self.0 |= BackendFlags::PAGINATION;
        }
        self
    }

    pub(super) const fn openapi(mut self, val: bool) -> Self {
        if val {
            self.0 |= BackendFlags::OPENAPI;
        }
        self
    }

    pub(super) const fn rate_limit(mut self, val: bool) -> Self {
        if val {
            self.0 |= BackendFlags::RATE_LIMIT;
        }
        self
    }

    pub(super) const fn problem_details(mut self, val: bool) -> Self {
        if val {
            self.0 |= BackendFlags::PROBLEM_DETAILS;
        }
        self
    }

    pub(super) const fn idempotency_key(mut self, val: bool) -> Self {
        if val {
            self.0 |= BackendFlags::IDEMPOTENCY_KEY;
        }
        self
    }

    pub(super) const fn build(self) -> BackendFlags {
        BackendFlags(self.0)
    }
}
