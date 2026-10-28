/// Hides an email's local part except its first character:
/// "jane@example.com" → "j***@example.com". Input without an `@` is
/// returned unchanged.
pub fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) if !local.is_empty() => {
            format!("{}***@{}", &local[..1], domain)
        }
        _ => email.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_the_local_part() {
        assert_eq!(mask_email("jane@example.com"), "j***@example.com");
    }
}
