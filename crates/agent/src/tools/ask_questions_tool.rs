use crate::{AgentTool, ToolCallEventStream, ToolInput};
use agent_client_protocol::schema as acp;
use acp_thread::UserQuestion;
use anyhow::Result;
use gpui::{App, SharedString, Task};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A single question the agent wants to ask the user.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct QuestionInput {
    /// The question to present to the user.
    pub text: String,
    /// Optional list of suggested answers. The user may also provide a custom answer.
    #[serde(default)]
    pub options: Vec<String>,
}

/// Asks the user a series of questions before proceeding.
///
/// Use this tool during the planning phase to collect context, preferences, or
/// clarifications needed before executing. The user will be shown each question
/// one at a time and may either select a suggested answer or type a custom one.
/// The tool returns the user's answers so you can incorporate them into your plan.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AskQuestionsToolInput {
    /// The list of questions to ask the user in order.
    pub questions: Vec<QuestionInput>,
}

pub struct AskQuestionsTool;

impl AgentTool for AskQuestionsTool {
    type Input = AskQuestionsToolInput;
    type Output = String;

    const NAME: &'static str = "ask_questions";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Think
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input {
            Ok(input) if input.questions.len() == 1 => "Asking a question".into(),
            Ok(input) => format!("Asking {} questions", input.questions.len()).into(),
            Err(_) => "Asking questions".into(),
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input
                .recv()
                .await
                .map_err(|e| format!("Failed to receive tool input: {e}"))?;

            let questions: Vec<UserQuestion> = input
                .questions
                .into_iter()
                .map(|q| UserQuestion {
                    text: q.text,
                    options: q.options,
                })
                .collect();

            let answers = cx
                .update(|cx| event_stream.ask_user_questions(questions, cx))
                .await
                .map_err(|e| format!("Questions cancelled: {e}"))?;

            let formatted = answers
                .iter()
                .enumerate()
                .map(|(i, answer)| format!("Answer {}: {}", i + 1, answer))
                .collect::<Vec<_>>()
                .join("\n");

            Ok(formatted)
        })
    }

    fn replay(
        &self,
        _input: Self::Input,
        _output: Self::Output,
        _event_stream: ToolCallEventStream,
        _cx: &mut App,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
