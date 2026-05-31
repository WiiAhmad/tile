#[cfg(test)]
#[derive(Debug, Clone)]
pub struct ToolLoopBudget {
    pub max_failures: usize,
    pub failures: usize,
}

#[cfg(test)]
impl ToolLoopBudget {
    pub fn new(max_failures: usize) -> Self {
        Self { max_failures, failures: 0 }
    }

    pub fn record_failure_and_can_continue(&mut self) -> bool {
        self.failures += 1;
        self.failures < self.max_failures
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_failures_until_budget_is_reached() {
        let mut budget = ToolLoopBudget::new(3);
        assert!(budget.record_failure_and_can_continue());
        assert!(budget.record_failure_and_can_continue());
        assert!(!budget.record_failure_and_can_continue());
        assert_eq!(budget.failures, 3);
    }
}
