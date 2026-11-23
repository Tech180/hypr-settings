/// A square QR code: `dark[y * size + x]` is true for a dark module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrMatrix {
    pub size: usize,
    pub dark: Vec<bool>,
}

impl QrMatrix {
    pub fn is_dark(&self, x: usize, y: usize) -> bool {
        x < self.size && y < self.size && self.dark[y * self.size + x]
    }
}

/// The `WIFI:` payload phones read to join a network. Never log it: it carries the password.
pub fn wifi_qr_payload(ssid: &str, password: &str, security: &str) -> String {
    let upper = security.to_uppercase();
    let auth = if password.is_empty() || upper.is_empty() || upper == "--" || upper.contains("OPEN")
    {
        "nopass"
    } else if upper.contains("WEP") {
        "WEP"
    } else {
        "WPA"
    };
    format!(
        "WIFI:T:{auth};S:{};P:{};H:false;;",
        escape(ssid),
        escape(password)
    )
}

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        if matches!(ch, '\\' | ';' | ',' | '"' | ':') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wpa_payload_escapes_special_characters() {
        assert_eq!(
            wifi_qr_payload("My;Net", "pa:ss\\word", "wpa-psk"),
            r"WIFI:T:WPA;S:My\;Net;P:pa\:ss\\word;H:false;;"
        );
    }

    #[test]
    fn open_networks_and_empty_secrets_are_nopass() {
        assert!(wifi_qr_payload("Cafe", "", "wpa-psk").starts_with("WIFI:T:nopass;"));
        assert!(wifi_qr_payload("Cafe", "x", "--").starts_with("WIFI:T:nopass;"));
        assert!(wifi_qr_payload("Old", "x", "WEP").starts_with("WIFI:T:WEP;"));
    }

    #[test]
    fn matrix_bounds_are_safe() {
        let m = QrMatrix {
            size: 2,
            dark: vec![true, false, false, true],
        };
        assert!(m.is_dark(0, 0) && m.is_dark(1, 1));
        assert!(!m.is_dark(1, 0) && !m.is_dark(5, 5));
    }
}
