const SERVICE: &str = "keynova";
const REF_PREFIX: &str = "keyring:keynova:";

pub fn secret_reference_for_key(key: &str) -> String {
    format!("{REF_PREFIX}{key}")
}

pub fn is_secret_reference(value: &str) -> bool {
    value.trim().starts_with(REF_PREFIX)
}

pub fn store_secret(key: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let entry = entry_for_key(key)?;
    if trimmed.is_empty() {
        let _ = entry.delete_credential();
        return Ok(String::new());
    }
    entry
        .set_password(value)
        .map_err(|e| format!("failed to store secret '{key}' in OS keychain: {e}"))?;
    Ok(secret_reference_for_key(key))
}

pub fn load_secret(key: &str) -> Result<String, String> {
    entry_for_key(key)?
        .get_password()
        .map_err(|e| format!("failed to load secret '{key}' from OS keychain: {e}"))
}

fn entry_for_key(key: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, key)
        .map_err(|e| format!("failed to open OS keychain entry '{key}': {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_references_are_stable_and_detectable() {
        let reference = secret_reference_for_key("ai.api_key");

        assert_eq!(reference, "keyring:keynova:ai.api_key");
        assert!(is_secret_reference(&reference));
        assert!(!is_secret_reference("plain-secret"));
    }
}
