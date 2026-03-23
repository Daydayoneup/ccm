use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "disable-model-invocation")]
    pub disable_model_invocation: Option<bool>,
    #[serde(rename = "user-invocable")]
    pub user_invocable: Option<bool>,
    #[serde(rename = "allowed-tools")]
    pub allowed_tools: Option<Vec<String>>,
    pub model: Option<String>,
    pub effort: Option<String>,
    // Do NOT use #[serde(skip)] — this field must survive Tauri IPC deserialization from frontend.
    // We exclude it from YAML output via custom logic in serialize_frontmatter().
    #[serde(default)]
    pub extra_yaml: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillFrontmatterData {
    pub frontmatter: SkillFrontmatter,
    pub body: String,
}

const KNOWN_KEYS: &[&str] = &[
    "name",
    "description",
    "disable-model-invocation",
    "user-invocable",
    "allowed-tools",
    "model",
    "effort",
];

/// Parse a SKILL.md string into frontmatter + body.
/// If there is no leading `---` block the frontmatter is empty and the full
/// content is returned as the body.
pub fn parse_frontmatter(content: &str) -> Result<SkillFrontmatterData, String> {
    if !content.starts_with("---") {
        return Ok(SkillFrontmatterData {
            frontmatter: SkillFrontmatter::default(),
            body: content.to_string(),
        });
    }

    // Skip the opening `---` line and look for a closing `---`
    let after_open = &content[3..];
    // The closing marker must appear as `---` at the start of a line
    let close_pos = after_open
        .find("\n---")
        .ok_or_else(|| "Frontmatter closing `---` not found".to_string())?;

    // yaml_str: strip the leading newline that follows the opening `---`
    let yaml_str = after_open[..close_pos].trim_start_matches('\n');
    // Skip past `\n---` (4 bytes), then skip `\n\n` (blank separator line) or just `\n`
    let rest = &after_open[close_pos + 4..];
    let body = if let Some(s) = rest.strip_prefix("\n\n") {
        s.to_string()
    } else {
        rest.strip_prefix('\n').unwrap_or(rest).to_string()
    };

    // Parse the YAML into a generic map so we can capture ALL keys
    let all_keys: BTreeMap<String, serde_yaml::Value> =
        serde_yaml::from_str(yaml_str).map_err(|e| format!("YAML parse error: {}", e))?;

    // Parse into the typed struct (known fields)
    let mut frontmatter: SkillFrontmatter =
        serde_yaml::from_str(yaml_str).map_err(|e| format!("YAML parse error: {}", e))?;

    // Collect unknown keys
    let mut extra_map: BTreeMap<String, serde_yaml::Value> = BTreeMap::new();
    for (k, v) in &all_keys {
        if !KNOWN_KEYS.contains(&k.as_str()) {
            extra_map.insert(k.clone(), v.clone());
        }
    }

    if !extra_map.is_empty() {
        let extra_str = serde_yaml::to_string(&extra_map)
            .map_err(|e| format!("Failed to serialize extra YAML: {}", e))?;
        frontmatter.extra_yaml = Some(extra_str);
    }

    Ok(SkillFrontmatterData { frontmatter, body })
}

/// Serialize a `SkillFrontmatter` and body back to a markdown string.
/// The `extra_yaml` field is excluded from the YAML block and its contents
/// are merged back directly.
pub fn serialize_frontmatter(frontmatter: &SkillFrontmatter, body: &str) -> Result<String, String> {
    // Serialize to a serde_yaml Value first
    let value = serde_yaml::to_value(frontmatter)
        .map_err(|e| format!("Failed to serialize frontmatter: {}", e))?;

    // Convert to a BTreeMap so we can manipulate individual keys
    let mut map: BTreeMap<String, serde_yaml::Value> = match value {
        serde_yaml::Value::Mapping(m) => m
            .into_iter()
            .filter_map(|(k, v)| {
                if let serde_yaml::Value::String(key) = k {
                    Some((key, v))
                } else {
                    None
                }
            })
            .collect(),
        _ => BTreeMap::new(),
    };

    // Remove null values and the extra_yaml key itself
    map.retain(|k, v| {
        k != "extra_yaml" && k != "extra-yaml" && !matches!(v, serde_yaml::Value::Null)
    });

    // Merge extra_yaml fields back
    if let Some(ref extra_str) = frontmatter.extra_yaml {
        if let Ok(extra_map) =
            serde_yaml::from_str::<BTreeMap<String, serde_yaml::Value>>(extra_str)
        {
            for (k, v) in extra_map {
                map.insert(k, v);
            }
        }
    }

    if map.is_empty() {
        return Ok(body.to_string());
    }

    let yaml = serde_yaml::to_string(&map)
        .map_err(|e| format!("Failed to serialize YAML map: {}", e))?;
    let yaml = yaml.trim_end_matches('\n');

    Ok(format!("---\n{}\n---\n\n{}", yaml, body))
}

/// Validate that `version` is a valid semver string.
pub fn validate_semver(version: &str) -> Result<(), String> {
    semver::Version::parse(version)
        .map(|_| ())
        .map_err(|e| format!("Invalid semver '{}': {}", version, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_frontmatter() {
        let content = "---\nname: my-skill\ndescription: Does something cool\n---\n\n# Body here\n";
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.frontmatter.name.as_deref(), Some("my-skill"));
        assert_eq!(
            result.frontmatter.description.as_deref(),
            Some("Does something cool")
        );
        assert_eq!(result.body, "# Body here\n");
        assert!(result.frontmatter.extra_yaml.is_none());
    }

    #[test]
    fn test_parse_with_extra_yaml() {
        let content = "---\nname: my-skill\ncontext: some-context\nagent: my-agent\nargument-hint: \"<arg>\"\n---\n\nBody text\n";
        let result = parse_frontmatter(content).unwrap();
        assert_eq!(result.frontmatter.name.as_deref(), Some("my-skill"));

        let extra = result.frontmatter.extra_yaml.as_ref().expect("extra_yaml should be set");
        assert!(extra.contains("context"), "extra_yaml should contain 'context'");
        assert!(extra.contains("agent"), "extra_yaml should contain 'agent'");
        assert!(extra.contains("argument-hint"), "extra_yaml should contain 'argument-hint'");
        assert_eq!(result.body, "Body text\n");
    }

    #[test]
    fn test_parse_with_allowed_tools() {
        let content = "---\nname: tooled\nallowed-tools:\n  - Bash\n  - Read\n  - Write\n---\n\nUse tools.\n";
        let result = parse_frontmatter(content).unwrap();
        let tools = result.frontmatter.allowed_tools.as_ref().unwrap();
        assert_eq!(tools, &["Bash", "Read", "Write"]);
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let content = "# Just a heading\n\nSome content here.\n";
        let result = parse_frontmatter(content).unwrap();
        assert!(result.frontmatter.name.is_none());
        assert!(result.frontmatter.description.is_none());
        assert_eq!(result.body, content);
    }

    #[test]
    fn test_serialize_roundtrip() {
        let original = "---\nname: roundtrip\ndescription: testing roundtrip\nmodel: claude-3\n---\n\nContent body.\n";
        let parsed = parse_frontmatter(original).unwrap();
        let serialized = serialize_frontmatter(&parsed.frontmatter, &parsed.body).unwrap();
        let reparsed = parse_frontmatter(&serialized).unwrap();

        assert_eq!(reparsed.frontmatter.name, parsed.frontmatter.name);
        assert_eq!(reparsed.frontmatter.description, parsed.frontmatter.description);
        assert_eq!(reparsed.frontmatter.model, parsed.frontmatter.model);
        assert_eq!(reparsed.body, parsed.body);
    }

    #[test]
    fn test_serialize_preserves_extra_yaml() {
        let content = "---\nname: extras\ncontext: special-context\nagent: helper\n---\n\nBody.\n";
        let parsed = parse_frontmatter(content).unwrap();
        assert!(parsed.frontmatter.extra_yaml.is_some());

        let serialized = serialize_frontmatter(&parsed.frontmatter, &parsed.body).unwrap();
        let reparsed = parse_frontmatter(&serialized).unwrap();

        assert_eq!(reparsed.frontmatter.name.as_deref(), Some("extras"));

        let extra = reparsed.frontmatter.extra_yaml.as_ref().expect("extra_yaml should survive roundtrip");
        assert!(extra.contains("context"));
        assert!(extra.contains("agent"));
    }

    #[test]
    fn test_validate_semver_valid() {
        assert!(validate_semver("1.0.0").is_ok());
        assert!(validate_semver("0.1.0").is_ok());
        assert!(validate_semver("10.20.30").is_ok());
    }

    #[test]
    fn test_validate_semver_invalid() {
        assert!(validate_semver("1.0").is_err());
        assert!(validate_semver("abc").is_err());
        assert!(validate_semver("").is_err());
    }
}
