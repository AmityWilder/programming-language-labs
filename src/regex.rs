use std::str::pattern::{Pattern, ReverseSearcher, SearchStep, Searcher};

#[must_use]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct FauxRegex<'a> {
    haystack: &'a str,
    matched_len: usize,
}

impl<'a> FauxRegex<'a> {
    pub const fn within(haystack: &'a str) -> Self {
        Self {
            haystack,
            matched_len: 0,
        }
    }

    #[must_use]
    fn unmatched(&self) -> &'a str {
        debug_assert!(
            self.matched_len <= self.haystack.len(),
            "matched_len should be within haystack\n matched_len: {}\n haystack.len(): {}",
            self.matched_len,
            self.haystack.len()
        );
        self.haystack
            .get(self.matched_len..)
            .expect("matched_len should be within haystack and not within a UTF-8 character")
    }

    #[must_use]
    pub fn matched(&self) -> &'a str {
        debug_assert!(
            self.matched_len <= self.haystack.len(),
            "matched_len should be within haystack\n matched_len: {}\n haystack.len(): {}",
            self.matched_len,
            self.haystack.len()
        );
        self.haystack
            .get(..self.matched_len)
            .expect("matched_len should be within haystack and not within a UTF-8 character")
    }

    #[expect(clippy::unnecessary_wraps, reason = "for convenience")]
    pub const fn end(&mut self) -> Option<&mut Self> {
        Some(self)
    }

    #[track_caller]
    fn include_in_match(&mut self, len: usize) {
        let new_len = self.matched_len.strict_add(len);
        debug_assert!(
            new_len <= self.haystack.len(),
            "matched_len should always be within haystack\n matched_len: {}\n haystack.len(): {}",
            new_len,
            self.haystack.len()
        );
        self.matched_len = new_len;
    }

    pub fn exactly<P>(&mut self, pat: P) -> Option<&mut Self>
    where
        P: Pattern,
    {
        if let SearchStep::Match(_, len) = pat.into_searcher(self.unmatched()).next() {
            self.include_in_match(len);
            Some(self)
        } else {
            None
        }
    }

    /// `?`
    pub fn optional<P>(&mut self, pat: P) -> &mut Self
    where
        P: Pattern,
    {
        if let SearchStep::Match(_, len) = pat.into_searcher(self.unmatched()).next() {
            self.include_in_match(len);
        }
        self
    }

    /// `*`
    pub fn repeat<P>(&mut self, pat: P) -> &mut Self
    where
        P: Pattern,
    {
        let mut searcher = pat.into_searcher(self.unmatched());
        while let SearchStep::Match(start, end) = searcher.next() {
            self.include_in_match(end.strict_sub(start));
        }
        self
    }

    /// `{at_least, at_most}`
    ///
    /// TIP: prefer [`Self::repeat`] if calling with `at_least=0` and `at_most=None`
    #[expect(dead_code, reason = "available for future use")]
    pub fn repeat_n<P>(
        &mut self,
        pat: P,
        at_least: usize,
        at_most: Option<usize>,
    ) -> Option<&mut Self>
    where
        P: Pattern,
    {
        let mut searcher = pat.into_searcher(self.unmatched());
        let mut repeated_len: usize = 0;
        for _ in 0..at_least {
            if let SearchStep::Match(start, end) = searcher.next() {
                repeated_len = repeated_len.strict_add(end.strict_sub(start));
            } else {
                return None; // not enough to match
            }
        }
        self.include_in_match(repeated_len);
        if let Some(at_most) = at_most {
            for _ in at_least..at_most {
                if let SearchStep::Match(start, end) = searcher.next() {
                    self.include_in_match(end.strict_sub(start));
                } else {
                    break;
                }
            }
        } else {
            while let SearchStep::Match(start, end) = searcher.next() {
                self.include_in_match(end.strict_sub(start));
            }
        }
        Some(self)
    }

    /// `(?: ... )?`
    pub fn opt_group<G>(&mut self, group_pat: G) -> &mut Self
    where
        G: FnOnce(&mut Self) -> Option<&mut Self>,
    {
        let mut inner = self.clone();
        if let Some(Self { matched_len, .. }) = group_pat(&mut inner) {
            self.matched_len = *matched_len;
        }
        self
    }

    /// `(?<= ... )`
    pub fn lookbehind<P>(&mut self, pat: P) -> Option<&mut Self>
    where
        P: for<'b> Pattern<Searcher<'b>: ReverseSearcher<'b>>,
    {
        self.matched().ends_with(pat).then_some(self)
    }
}

/// `\w` (unicode, not ascii)
pub fn word_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

/// `\d` (unicode, not ascii)
pub fn digit_char(ch: char) -> bool {
    ch.is_numeric()
}
