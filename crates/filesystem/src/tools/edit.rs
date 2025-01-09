use mcp::protocol::{Annotated, TextContent};
use serde_json::json;
use similar::{ChangeTag, TextDiff};
use std::pin::Pin;
use std::sync::Arc;
use std::{collections::HashMap, future::Future};

use super::{EditOperation, Tool};
use crate::security::PathValidator;

pub struct EditFileTool {
    validator: Arc<PathValidator>,
}

impl EditFileTool {
    pub fn new(validator: Arc<PathValidator>) -> Self {
        Self { validator }
    }

    fn normalize_line_endings(text: &str) -> String {
        text.replace("\r\n", "\n")
    }

    fn create_diff(original: &str, modified: &str) -> String {
        let diff = TextDiff::from_lines(original, modified);

        let mut diff_output = String::new();
        for change in diff.iter_all_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            diff_output.push_str(&format!("{}{}", sign, change));
        }
        diff_output
    }

    async fn apply_edits(
        &self,
        content: String,
        edits: Vec<EditOperation>,
    ) -> crate::Result<String> {
        let mut result = content;

        for edit in edits {
            let old_normalized = Self::normalize_line_endings(&edit.old_text);
            let new_normalized = Self::normalize_line_endings(&edit.new_text);

            if !result.contains(&old_normalized) {
                // Try line-by-line matching if exact match fails
                let old_lines: Vec<&str> = old_normalized.lines().collect();
                let content_lines: Vec<&str> = result.lines().collect();

                let mut found_match = false;
                'outer: for i in 0..=content_lines.len().saturating_sub(old_lines.len()) {
                    let mut matches = true;
                    for (j, old_line) in old_lines.iter().enumerate() {
                        if i + j >= content_lines.len()
                            || content_lines[i + j].trim() != old_line.trim()
                        {
                            matches = false;
                            break;
                        }
                    }
                    if matches {
                        // We found a matching section, now format the replacement
                        let replacement = if i > 0 {
                            // Preserve the indentation of the first line
                            let indent = content_lines[i]
                                .chars()
                                .take_while(|c| c.is_whitespace())
                                .collect::<String>();
                            new_normalized
                                .lines()
                                .enumerate()
                                .map(|(idx, line)| {
                                    if idx == 0 {
                                        format!("{}{}", indent, line.trim_start())
                                    } else {
                                        line.to_string()
                                    }
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        } else {
                            new_normalized.clone()
                        };

                        // Split content into lines, replace the matching section, and rejoin
                        let mut new_lines = content_lines.to_vec();
                        new_lines.splice(i..i + old_lines.len(), replacement.lines());
                        result = new_lines.join("\n");
                        found_match = true;
                        break 'outer;
                    }
                }

                if !found_match {
                    return Err(crate::error::Error::InvalidPath(format!(
                        "Could not find match for text to replace:\n{}",
                        edit.old_text
                    )));
                }
            } else {
                // Exact match found, perform simple replacement
                result = result.replace(&old_normalized, &new_normalized);
            }
        }

        Ok(result)
    }
}

impl Tool for EditFileTool {
    fn name(&self) -> &'static str {
        "edit_file"
    }

    fn description(&self) -> &'static str {
        "Make line-based edits to a text file. Each edit replaces exact line sequences \
        with new content. Returns a git-style diff showing the changes made. \
        Only works within allowed directories."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["path", "edits"],
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to edit"
                },
                "edits": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["old_text", "new_text"],
                        "properties": {
                            "old_text": {
                                "type": "string",
                                "description": "Text to search for and replace"
                            },
                            "new_text": {
                                "type": "string",
                                "description": "Text to replace with"
                            }
                        }
                    },
                    "description": "List of text replacements to perform"
                },
                "dry_run": {
                    "type": "boolean",
                    "description": "If true, show changes without applying them",
                    "default": false
                }
            }
        })
    }

    fn run<'a>(
        &'a self,
        params: Option<HashMap<String, serde_json::Value>>,
    ) -> Pin<Box<dyn Future<Output = crate::Result<TextContent>> + Send + 'a>> {
        Box::pin(async move {
            let params = params
                .ok_or_else(|| crate::error::Error::InvalidPath("No parameters provided".into()))?;

            // Parse path
            let path = params
                .get("path")
                .ok_or_else(|| crate::error::Error::InvalidPath("No path provided".into()))?
                .as_str()
                .ok_or_else(|| crate::error::Error::InvalidPath("Path must be a string".into()))?;

            // Parse edits
            let edits = params
                .get("edits")
                .ok_or_else(|| crate::error::Error::InvalidPath("No edits provided".into()))?
                .as_array()
                .ok_or_else(|| crate::error::Error::InvalidPath("Edits must be an array".into()))?;

            let edits: Vec<EditOperation> = edits
                .iter()
                .map(|edit| {
                    let obj = edit.as_object().ok_or_else(|| {
                        crate::error::Error::InvalidPath("Edit must be an object".into())
                    })?;

                    Ok(EditOperation {
                        old_text: obj
                            .get("old_text")
                            .ok_or_else(|| {
                                crate::error::Error::InvalidPath("Missing old_text in edit".into())
                            })?
                            .as_str()
                            .ok_or_else(|| {
                                crate::error::Error::InvalidPath("old_text must be a string".into())
                            })?
                            .to_string(),
                        new_text: obj
                            .get("new_text")
                            .ok_or_else(|| {
                                crate::error::Error::InvalidPath("Missing new_text in edit".into())
                            })?
                            .as_str()
                            .ok_or_else(|| {
                                crate::error::Error::InvalidPath("new_text must be a string".into())
                            })?
                            .to_string(),
                    })
                })
                .collect::<crate::Result<Vec<_>>>()?;

            // Parse dry_run flag
            let dry_run = params
                .get("dry_run")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            // Validate path
            let path = self.validator.validate_path(path).await?;

            // Read current content
            let original_content = tokio::fs::read_to_string(&path).await?;

            // Apply edits
            let new_content = self.apply_edits(original_content.clone(), edits).await?;

            // Generate diff
            let diff = Self::create_diff(&original_content, &new_content);

            // Write changes if not dry run
            if !dry_run {
                tokio::fs::write(&path, new_content).await?;
            }

            Ok(TextContent {
                text: format!(
                    "Changes {}:\n{}",
                    if dry_run { "to be made" } else { "made" },
                    diff
                ),
                annotated: Annotated { annotations: None },
            })
        })
    }
}

// Remove this impl block entirely
// impl ToolExecute for EditFileTool { ... }
