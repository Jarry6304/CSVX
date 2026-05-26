use super::Profile;

/// Loose, fast path: filename glob + header_contains.
/// Signature check (strict) is done after a profile is picked — drift is a warning,
/// not a rejection (per spec §6).
pub fn matches(profile: &Profile, filename: &str, header_row: &[String]) -> bool {
    if let Some(pattern) = &profile.match_rules.filename {
        if !glob_match(pattern, filename) {
            return false;
        }
    }
    let header_joined = header_row.join("|");
    profile
        .match_rules
        .header_contains
        .iter()
        .all(|needle| header_joined.contains(needle.as_str()))
}

/// Minimal glob matching against a filename: supports `*` and `?`. No `**` or `[..]`.
/// Tested against patterns like `*發電量*.xlsx`.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    glob_recurse(pattern.as_bytes(), name.as_bytes())
}

fn glob_recurse(pat: &[u8], txt: &[u8]) -> bool {
    // Byte-level matching is fine here because both inputs are matched literally
    // and `*`/`?` are ASCII; multi-byte UTF-8 characters always pass through unchanged.
    let mut pi = 0usize;
    let mut ti = 0usize;
    let mut star: Option<(usize, usize)> = None;

    while ti < txt.len() {
        if pi < pat.len() {
            match pat[pi] {
                b'*' => {
                    star = Some((pi, ti));
                    pi += 1;
                    continue;
                }
                b'?' => {
                    pi += 1;
                    ti += 1;
                    continue;
                }
                b => {
                    if b == txt[ti] {
                        pi += 1;
                        ti += 1;
                        continue;
                    }
                }
            }
        }
        if let Some((sp, st)) = star {
            pi = sp + 1;
            ti = st + 1;
            star = Some((sp, st + 1));
            continue;
        }
        return false;
    }

    while pi < pat.len() && pat[pi] == b'*' {
        pi += 1;
    }
    pi == pat.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_basic() {
        assert!(glob_match("*.xlsx", "x.xlsx"));
        assert!(glob_match("*發電量*.xlsx", "202604_案場發電量_v1.xlsx"));
        assert!(!glob_match("*發電量*.xlsx", "202604_案場營收_v1.xlsx"));
        assert!(glob_match("foo?.csv", "foo1.csv"));
        assert!(!glob_match("foo?.csv", "foo12.csv"));
    }

    #[test]
    fn glob_anchored() {
        assert!(glob_match("a*z", "az"));
        assert!(glob_match("a*z", "abcz"));
        assert!(!glob_match("a*z", "abc"));
    }
}
