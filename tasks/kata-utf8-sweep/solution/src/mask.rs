/// Hides an email's local part except its first character:
/// "jane@example.com" → "j***@example.com". Input without an `@` is
/// returned unchanged.
pub fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) if !local.is_empty() => {
            let first = local.chars().next().unwrap();
            format!("{first}***@{domain}")
        }
        _ => email.to_string(),
    }
}
