use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed properties from a SKILL.md frontmatter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillProperties {
    pub name: String,
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,

    #[serde(rename = "allowed-tools", skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_yaml_ng::Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_props() -> SkillProperties {
        SkillProperties {
            name: "foo".to_string(),
            description: "bar".to_string(),
            license: None,
            compatibility: None,
            allowed_tools: None,
            metadata: None,
        }
    }

    fn full_props() -> SkillProperties {
        let mut meta = HashMap::new();
        meta.insert(
            "env".to_string(),
            serde_yaml_ng::Value::String("prod".to_string()),
        );
        SkillProperties {
            name: "my-skill".to_string(),
            description: "A test skill".to_string(),
            license: Some("MIT".to_string()),
            compatibility: Some("claude-3".to_string()),
            allowed_tools: Some("Bash, Read".to_string()),
            metadata: Some(meta),
        }
    }

    #[test]
    fn construct_with_required_fields_only() {
        let sp = minimal_props();
        assert_eq!(sp.name, "foo");
        assert_eq!(sp.description, "bar");
        assert!(sp.license.is_none());
        assert!(sp.metadata.is_none());
    }

    #[test]
    fn construct_with_all_fields() {
        let sp = full_props();
        assert_eq!(sp.name, "my-skill");
        assert_eq!(sp.license, Some("MIT".to_string()));
        assert_eq!(sp.allowed_tools, Some("Bash, Read".to_string()));
        assert!(sp.metadata.is_some());
    }

    #[test]
    fn serialize_json_omits_none_fields() {
        let sp = minimal_props();
        let v = serde_json::to_value(&sp).unwrap();
        assert_eq!(v["name"], "foo");
        assert_eq!(v["description"], "bar");
        assert!(v.get("license").is_none());
        assert!(v.get("compatibility").is_none());
        assert!(v.get("allowed-tools").is_none());
        assert!(v.get("metadata").is_none());
    }

    #[test]
    fn serialize_json_includes_license_when_some() {
        let mut sp = minimal_props();
        sp.license = Some("MIT".to_string());
        let v = serde_json::to_value(&sp).unwrap();
        assert_eq!(v["license"], "MIT");
    }

    #[test]
    fn serialize_json_includes_all_optional_fields() {
        let sp = full_props();
        let v = serde_json::to_value(&sp).unwrap();
        assert_eq!(v["name"], "my-skill");
        assert_eq!(v["license"], "MIT");
        assert_eq!(v["compatibility"], "claude-3");
        assert_eq!(v["allowed-tools"], "Bash, Read");
        assert!(v.get("metadata").is_some());
    }

    #[test]
    fn serialize_json_excludes_metadata_when_none() {
        let sp = minimal_props();
        let v = serde_json::to_value(&sp).unwrap();
        assert!(v.get("metadata").is_none());
    }

    #[test]
    fn serialize_json_includes_metadata_when_some() {
        let sp = full_props();
        let v = serde_json::to_value(&sp).unwrap();
        let meta = v.get("metadata").unwrap();
        assert_eq!(meta["env"], "prod");
    }

    #[test]
    fn deserialize_from_yaml_all_fields() {
        let yaml = r#"
name: my-skill
description: A test skill
license: MIT
compatibility: claude-3
allowed-tools: Bash, Read
metadata:
  env: prod
"#;
        let sp: SkillProperties = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(sp.name, "my-skill");
        assert_eq!(sp.description, "A test skill");
        assert_eq!(sp.license, Some("MIT".to_string()));
        assert_eq!(sp.compatibility, Some("claude-3".to_string()));
        assert_eq!(sp.allowed_tools, Some("Bash, Read".to_string()));
        let meta = sp.metadata.unwrap();
        assert_eq!(
            meta["env"],
            serde_yaml_ng::Value::String("prod".to_string())
        );
    }

    #[test]
    fn allowed_tools_kebab_case_round_trip() {
        let yaml = "name: test\ndescription: desc\nallowed-tools: Bash, Read\n";
        let sp: SkillProperties = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(sp.allowed_tools, Some("Bash, Read".to_string()));
        let json = serde_json::to_value(&sp).unwrap();
        assert_eq!(json["allowed-tools"], "Bash, Read");
    }

    #[test]
    fn field_accessors_name_and_description() {
        let sp = minimal_props();
        assert_eq!(sp.name, "foo");
        assert_eq!(sp.description, "bar");
    }

    #[test]
    fn field_accessor_allowed_tools() {
        let sp = full_props();
        assert_eq!(sp.allowed_tools.as_deref(), Some("Bash, Read"));
    }

    #[test]
    fn field_accessor_metadata() {
        let sp = full_props();
        let meta = sp.metadata.as_ref().unwrap();
        assert_eq!(
            meta["env"],
            serde_yaml_ng::Value::String("prod".to_string())
        );
    }

    #[test]
    fn deserialize_yaml_required_fields_only() {
        let yaml = "name: foo\ndescription: bar\n";
        let sp: SkillProperties = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(sp.name, "foo");
        assert_eq!(sp.description, "bar");
        assert!(sp.license.is_none());
        assert!(sp.compatibility.is_none());
        assert!(sp.allowed_tools.is_none());
        assert!(sp.metadata.is_none());
    }

    #[test]
    fn partial_eq_identical() {
        let a = full_props();
        let b = full_props();
        assert_eq!(a, b);
    }

    #[test]
    fn partial_eq_different_field() {
        let a = minimal_props();
        let mut b = minimal_props();
        b.name = "different".to_string();
        assert_ne!(a, b);
    }

    #[test]
    fn deserialize_yaml_missing_name_fails() {
        let yaml = "description: bar\n";
        let result = serde_yaml_ng::from_str::<SkillProperties>(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_yaml_missing_description_fails() {
        let yaml = "name: foo\n";
        let result = serde_yaml_ng::from_str::<SkillProperties>(yaml);
        assert!(result.is_err());
    }
}
