use serde::de;
use std::convert::TryFrom;
use std::fmt;
use util::HostName;

/// A pattern matching domain names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsPattern(Option<HostName>);

impl DnsPattern {
    /// A pattern that matches every domain.
    pub fn wildcard() -> Self {
        DnsPattern(None)
    }

    /// Check if the given domain name matches this pattern.
    ///
    /// The matching follows the rules of [RFC 6265][1].
    ///
    /// [1]: https://datatracker.ietf.org/doc/html/rfc6265#section-5.1.3
    pub fn matches(&self, domain: &str) -> bool {
        let ours: &str =
            if let Some(dns) = &self.0 {
                dns.as_str()
            } else {
                return true
            };

        let mut theirs = domain.chars().rev();

        for ours in ours.chars().rev() {
            if let Some(theirs) = theirs.next() {
                if ours.to_ascii_lowercase() != theirs.to_ascii_lowercase() {
                    return false
                }
            } else {
                return false
            }
        }

        match theirs.next() {
            None    => true,
            Some(c) => c == '.'
        }
    }

    fn as_str(&self) -> &str {
        match &self.0 {
            None    => "",
            Some(d) => d.as_str()
        }
    }
}

impl TryFrom<&str> for DnsPattern {
    type Error = serde::de::value::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        if let Some(rem) = s.strip_prefix("*.") {
            if rem.is_empty() {
                return Ok(DnsPattern(None))
            }
            if let Ok(name) = HostName::try_from(rem) {
                return Ok(DnsPattern(Some(name)))
            }
        }
        Err(de::Error::custom("invalid DNS name pattern"))
    }
}

impl fmt::Display for DnsPattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "*.{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};
    use rand::{distributions::{Alphanumeric, DistString}, prelude::*};
    use std::convert::TryFrom;
    use super::DnsPattern;

    impl Arbitrary for DnsPattern {
        fn arbitrary(_: &mut Gen) -> Self {
            let mut r = rand::thread_rng();
            let mut v = Vec::new();

            for _ in 0 .. r.gen_range(0 .. 16) {
                let n = r.gen_range(1 .. 16);
                v.push(Alphanumeric.sample_string(&mut r, n))
            }

            let top = std::iter::repeat(())
                .map(|()| r.sample(Alphanumeric))
                .map(char::from)
                .filter(|c| c.is_ascii_alphabetic())
                .take(5);

            v.push(top.collect());
            v.insert(0, "*".to_string());

            DnsPattern::try_from(v.join(".").as_str()).unwrap()
        }
    }

    #[derive(Debug, Clone)]
    struct Ascii(String);

    impl Arbitrary for Ascii {
        fn arbitrary(g: &mut Gen) -> Self {
            let mut s = String::arbitrary(g);
            s.retain(|c| c.is_ascii_alphanumeric());
            Ascii(s)
        }
    }

    #[test]
    fn read_show_id() {
        fn prop(dns: DnsPattern) -> bool {
            dns == DnsPattern::try_from(dns.to_string().as_str()).unwrap()
        }
        quickcheck(prop as fn(_) -> bool)
    }

    #[test]
    fn matches_itself() {
        fn prop(dns: DnsPattern) -> bool {
            dns.matches(dns.as_str())
        }
        quickcheck(prop as fn(_) -> bool)
    }

    #[test]
    fn matches_domain_with_pattern_as_suffix() {
        fn prop(dns: DnsPattern, prefix: Vec<Ascii>) -> TestResult {
            if prefix.len() > 63 {
            }
            let mut domain = join(&prefix);
            domain.push_str(".");
            domain.push_str(dns.as_str());
            TestResult::from_bool(dns.matches(&domain))
        }
        quickcheck(prop as fn(_, _) -> TestResult)
    }

    #[test]
    fn prefix_needs_to_finish_with_dot() {
        fn prop(dns: DnsPattern, prefix: Vec<Ascii>) -> TestResult {
            if prefix.is_empty() || prefix.len() > 63 {
                return TestResult::discard()
            }
            let mut domain = join(&prefix);
            if domain.is_empty() {
                return TestResult::discard()
            }
            domain.push_str(dns.as_str());
            TestResult::from_bool(!dns.matches(&domain))
        }
        quickcheck(prop as fn(_, _) -> TestResult)
    }

    #[test]
    fn empty_pattern_matches_all() {
        fn prop(prefix: Vec<Ascii>) -> bool {
            let dom = join(&prefix);
            let pat = DnsPattern::wildcard();
            pat.matches(&dom)
        }
        quickcheck(prop as fn(_) -> bool)
    }

    fn join(parts: &[Ascii]) -> String {
        let mut result = String::new();
        for p in parts {
            result.push_str(&p.0);
        }
        result
    }
}
