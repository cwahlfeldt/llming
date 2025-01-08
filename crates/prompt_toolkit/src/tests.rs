// src/tests.rs
use crate::{PromptBuilder, PromptContext, Tool, ToolFunction};

#[test]
fn test_prompt_context_basic() {
    let mut context = PromptContext::new();
    context.set_system_message("System prompt");
    context.set_user_message("User question");

    let formatted = context.format();
    assert!(formatted.contains("System: System prompt"));
    assert!(formatted.contains("User: User question"));
}

#[test]
fn test_prompt_context_history() {
    let mut context = PromptContext::with_capacity(2);
    context.add_message("assistant", "Hello!");
    context.add_message("user", "Hi there");
    context.add_message("assistant", "How can I help?");

    let formatted = context.format();
    // Should only contain last 2 messages due to capacity
    assert!(!formatted.contains("Hello!"));
    assert!(formatted.contains("Hi there"));
    assert!(formatted.contains("How can I help?"));
}

#[test]
fn test_prompt_builder_basic() {
    let tool = Tool::new("calculator", "Basic calculator").add_function(ToolFunction::new(
        "add",
        "Adds two numbers",
        "a: number, b: number",
    ));

    let template = PromptBuilder::new()
        .with_template("Let's do math")
        .add_tool(tool)
        .build()
        .unwrap();

    let context = PromptContext::new();
    let formatted = template.format(&context);

    assert!(formatted.contains("Let's do math"));
    assert!(formatted.contains("calculator"));
    assert!(formatted.contains("add"));
}

#[test]
fn test_prompt_builder_with_variables() {
    let template = PromptBuilder::new()
        .with_template("Template")
        .with_variable("key", "value")
        .build()
        .unwrap();

    let context = PromptContext::new();
    let formatted = template.format(&context);

    assert!(formatted.contains("Template"));
}

#[test]
fn test_prompt_builder_missing_template() {
    let result = PromptBuilder::new().build();
    assert!(result.is_err());
}

#[test]
fn test_tool_formatting() {
    let tool = Tool::new("test_tool", "Test description").add_function(ToolFunction::new(
        "test_func",
        "Test function",
        "param1: string",
    ));

    let formatted = tool.format();
    assert!(formatted.contains("test_tool"));
    assert!(formatted.contains("Test description"));
    assert!(formatted.contains("test_func"));
    assert!(formatted.contains("param1: string"));
}
