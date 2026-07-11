use editor_model::Modifier;
use editor_resource::Resource;

fn parse_family_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|raw| {
            let trimmed = raw.trim();
            let unquoted = if (trimmed.starts_with('"') && trimmed.ends_with('"')
                || trimmed.starts_with('\'') && trimmed.ends_with('\''))
                && trimmed.len() >= 2
            {
                &trimmed[1..trimmed.len() - 1]
            } else {
                trimmed
            };
            unquoted.trim().to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn normalize(value: &str, resource: &Resource) -> Option<Modifier> {
    for candidate in parse_family_list(value) {
        if let Some(registered) = resource.font_registry.has_family_ci(&candidate) {
            return Some(Modifier::FontFamily {
                value: registered.to_string(),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use editor_resource::{FontFamily, FontFamilySource, FontWeight, Resource};

    fn make_resource_with(families: &[&str]) -> Resource {
        let mut r = Resource::new_test();
        let configs: Vec<FontFamily> = families
            .iter()
            .map(|name| FontFamily {
                name: (*name).to_string(),
                source: FontFamilySource::User,
                weights: vec![FontWeight {
                    value: 400,
                    hash: format!("h_{name}"),
                }],
            })
            .collect();
        r.set_fonts(configs);
        r
    }

    fn family(modifier: Option<Modifier>) -> Option<String> {
        match modifier? {
            Modifier::FontFamily { value } => Some(value),
            _ => None,
        }
    }

    #[test]
    fn single_registered_family_matches() {
        let r = make_resource_with(&["Pretendard"]);
        assert_eq!(
            family(normalize("Pretendard", &r)),
            Some("Pretendard".into())
        );
    }

    #[test]
    fn case_insensitive_returns_registered_name() {
        let r = make_resource_with(&["Pretendard"]);
        assert_eq!(
            family(normalize("pretendard", &r)),
            Some("Pretendard".into())
        );
        assert_eq!(
            family(normalize("PRETENDARD", &r)),
            Some("Pretendard".into())
        );
    }

    #[test]
    fn fallback_list_first_match_wins() {
        let r = make_resource_with(&["Pretendard"]);
        let m = normalize(r#""Arial", Pretendard, sans-serif"#, &r);
        assert_eq!(family(m), Some("Pretendard".into()));
    }

    #[test]
    fn double_quoted_family_unquoted() {
        let r = make_resource_with(&["Times New Roman"]);
        let m = normalize(r#""Times New Roman""#, &r);
        assert_eq!(family(m), Some("Times New Roman".into()));
    }

    #[test]
    fn single_quoted_family_unquoted() {
        let r = make_resource_with(&["Times New Roman"]);
        let m = normalize(r#"'Times New Roman'"#, &r);
        assert_eq!(family(m), Some("Times New Roman".into()));
    }

    #[test]
    fn no_match_returns_none() {
        let r = make_resource_with(&["Pretendard"]);
        assert!(normalize("Calibri, Arial, sans-serif", &r).is_none());
    }

    #[test]
    fn generic_keywords_skip_through() {
        let r = make_resource_with(&["Pretendard"]);
        let m = normalize("serif, sans-serif, Pretendard", &r);
        assert_eq!(family(m), Some("Pretendard".into()));
    }
}
