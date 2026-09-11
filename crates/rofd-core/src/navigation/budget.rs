//! Budget expanded model and diagnostic strings, not just encoded XML bytes.

use std::fmt::{self, Write};

use crate::{Error, Result};

pub(crate) struct StringBudget {
    remaining: u64,
    limit: u64,
}

impl StringBudget {
    pub(crate) fn new(limit: u64) -> Self {
        Self {
            remaining: limit,
            limit,
        }
    }

    pub(crate) fn used(&self) -> u64 {
        self.limit - self.remaining
    }

    fn exceeded() -> Error {
        Error::LimitExceeded("expanded navigation strings exceed max_entry_size".to_owned())
    }

    pub(crate) fn reserve(&mut self, amount: usize) -> Result<()> {
        let amount = u64::try_from(amount).map_err(|_| Self::exceeded())?;
        self.remaining = self
            .remaining
            .checked_sub(amount)
            .ok_or_else(Self::exceeded)?;
        Ok(())
    }

    pub(crate) fn copy(&mut self, value: &str) -> Result<String> {
        self.reserve(value.len())?;
        Ok(value.to_owned())
    }

    pub(crate) fn format(&mut self, arguments: fmt::Arguments<'_>) -> Result<String> {
        // Debug formatting can expand one input character into several escape
        // bytes. Check every formatted chunk before allocating it, rather than
        // building an unbounded temporary and charging its final length.
        let mut writer = BudgetWriter {
            budget: self,
            value: String::new(),
        };
        writer.write_fmt(arguments).map_err(|_| Self::exceeded())?;
        Ok(writer.value)
    }
}

struct BudgetWriter<'a> {
    budget: &'a mut StringBudget,
    value: String,
}

impl Write for BudgetWriter<'_> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.budget.reserve(value.len()).map_err(|_| fmt::Error)?;
        self.value.push_str(value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copied_and_formatted_strings_share_an_exact_byte_boundary() {
        let mut budget = StringBudget::new(10);
        assert_eq!(budget.copy("é").unwrap(), "é");
        assert_eq!(
            budget.format(format_args!("{:?}", '\u{7f}')).unwrap(),
            "'\\u{7f}'"
        );
        assert!(matches!(budget.copy("x"), Err(Error::LimitExceeded(_))));
        let mut insufficient = StringBudget::new(7);
        assert!(matches!(
            insufficient.format(format_args!("{:?}", '\u{7f}')),
            Err(Error::LimitExceeded(_))
        ));
    }
}
