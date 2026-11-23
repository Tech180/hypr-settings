//! Terse (`-t`) nmcli output.

/// Splits one line on unescaped `:`; a backslash escapes the next character.
pub fn split_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == ':' {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }
    fields.push(current);
    fields
}

/// One row per non-empty line.
pub fn table(output: &str) -> Vec<Vec<String>> {
    output
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(split_fields)
        .collect()
}

/// `KEY:value` lines (`nmcli -t ... show`); repeated keys keep every value.
#[derive(Debug, Default, Clone)]
pub struct Show(Vec<(String, String)>);

impl Show {
    pub fn parse(output: &str) -> Self {
        let mut rows = Vec::new();
        for line in output.lines() {
            let mut parts = split_fields(line).into_iter();
            let Some(key) = parts.next().filter(|k| !k.is_empty()) else {
                continue;
            };
            let value: Vec<String> = parts.collect();
            if value.is_empty() {
                continue;
            }
            let value = value.join(":");
            // nmcli prints `--` for "not set".
            rows.push((key, if value == "--" { String::new() } else { value }));
        }
        Self(rows)
    }

    /// The first value of `key`, or "".
    pub fn first(&self, key: &str) -> &str {
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .map_or("", |(_, v)| v.as_str())
    }

    /// The first non-empty of several keys.
    pub fn any(&self, keys: &[&str]) -> &str {
        keys.iter()
            .map(|k| self.first(k))
            .find(|v| !v.is_empty())
            .unwrap_or("")
    }

    /// The first value of every key starting with `prefix` (`IP4.ADDRESS[1]`, ...).
    pub fn prefixed(&self, prefix: &str) -> Vec<String> {
        let mut seen: Vec<&str> = Vec::new();
        let mut out = Vec::new();
        for (key, value) in &self.0 {
            if key.starts_with(prefix) && !value.is_empty() && !seen.contains(&key.as_str()) {
                seen.push(key);
                out.push(value.clone());
            }
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The leading integer of `"540 Mbit/s"`; 0 when there is none.
pub fn leading_number(text: &str) -> u32 {
    let digits: String = text
        .trim()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaped_colons_stay_inside_a_field() {
        assert_eq!(
            split_fields(r"*:My\:Net:80:AA\:BB\:CC"),
            ["*", "My:Net", "80", "AA:BB:CC"]
        );
        assert_eq!(split_fields("a::b"), ["a", "", "b"]);
    }

    #[test]
    fn show_keeps_values_with_colons_and_repeated_keys() {
        let show = Show::parse(
            "GENERAL.HWADDR:AA\\:BB\\:CC\\:DD\\:EE\\:FF\nIP4.DNS[1]:1.1.1.1\nIP4.DNS[2]:9.9.9.9\nIP4.GATEWAY:\nGENERAL.STATE:100 (connected)\n",
        );
        assert_eq!(show.first("GENERAL.HWADDR"), "AA:BB:CC:DD:EE:FF");
        assert_eq!(show.prefixed("IP4.DNS"), ["1.1.1.1", "9.9.9.9"]);
        assert_eq!(show.first("IP4.GATEWAY"), "");
        assert_eq!(
            show.any(&["IP4.GATEWAY", "GENERAL.STATE"]),
            "100 (connected)"
        );
    }

    #[test]
    fn leading_numbers() {
        assert_eq!(leading_number("540 Mbit/s"), 540);
        assert_eq!(leading_number(""), 0);
        assert_eq!(leading_number("unknown"), 0);
    }
}
