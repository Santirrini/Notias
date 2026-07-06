use time::OffsetDateTime;

pub fn time_now() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn time_now_returns_rfc3339() {
        let s = time_now();
        assert!(s.starts_with("20"), "year must be 20xx, got {s}");
        assert!(s.contains('T'), "must contain T separator, got {s}");
        assert!(s.ends_with('Z') || s.contains('+'), "must have timezone");
    }
}
