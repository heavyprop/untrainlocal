use std::collections::HashSet;

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
}

pub fn tokenize(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let lowercase = text.to_lowercase();
    words(&lowercase)
        .map(str::to_owned)
        .filter(|word| seen.insert(word.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenization_handles_case_duplicates_and_unicode() {
        assert_eq!(tokenize("Rust, RUST! café"), ["rust", "café"]);
    }
}
