use super::core::{Entity, RecordResponse, Relation};
use chrono::serde::ts_seconds;
use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use openai_api_rs::v1::api::Client;
use openai_api_rs::v1::chat_completion::{self, ChatCompletionRequest, FunctionCall, MessageRole};
use openai_api_rs::v1::common::{GPT3_5_TURBO, GPT4};
use poem_openapi::{Enum, Object};
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use validator::Validate;

lazy_static! {
    pub static ref UUID_REGEX: Regex =
        Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();
    // Only for predicted edge
    pub static ref PROMPT_TEMPLATE: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();

        m.insert("node_summary", "You need to execute the following instructions I send you: find the related information for the question, summarize the information you found and output a summary no more than 500 words, give me the sources of information. Notice: Please just return me the sentence 'I don't know what you say, it seems not to be a right question related with specific topic', if the question I send you is not related with medical concepts, such as {{entity_type}}.\n\nWhat's the {{entity_name}} which id is {{entity_id}}?");

        m.insert("edge_summary", "You need to execute the following instructions I send you: find the related information for the question, summarize the information you found and output a summary no more than 500 words, give me the sources of information. Notice: Please just return me the sentence 'I don't know what you say, it seems not to be a right question related with specific topic', if the question I send you is not related with medical concepts.\n\nWhat's the {{source_name}}[{{source_id}}, {{source_type}}] -> {{relation_type}} -> {{target_name}}[{{target_id}}, {{target_type}}?");

        m.insert("custom_question", "You need to execute the following instructions I send you: find the related information for the question, summarize the information you found and output a summary no more than 500 words, give me the sources of information. Notice: Please just return me the sentence 'I don't know what you say, it seems not to be a right question related with specific topic', if the question I send you is not related with medical concepts.\n\n{{custom_question}}");
        m
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Object)]
pub struct ExpandedRelation {
    pub relation: Relation,
    pub source: Entity,
    pub target: Entity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Enum)]
pub enum PromptTemplateCategoryEnum {
    NodeSummary,
    EdgeSummary,
    CustomQuestion,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Object)]
pub struct PromptTemplateCategory {
    value: PromptTemplateCategoryEnum,
}

impl From<String> for PromptTemplateCategory {
    fn from(v: String) -> Self {
        match v.as_str() {
            "node_summary" => PromptTemplateCategory {
                value: PromptTemplateCategoryEnum::NodeSummary,
            },
            "edge_summary" => PromptTemplateCategory {
                value: PromptTemplateCategoryEnum::EdgeSummary,
            },
            "custom_question" => PromptTemplateCategory {
                value: PromptTemplateCategoryEnum::CustomQuestion,
            },
            _ => panic!("Invalid prompt template category"),
        }
    }
}

impl Into<PromptTemplateCategoryEnum> for PromptTemplateCategory {
    fn into(self) -> PromptTemplateCategoryEnum {
        self.value
    }
}

impl Into<String> for PromptTemplateCategory {
    fn into(self) -> String {
        match self.value {
            PromptTemplateCategoryEnum::NodeSummary => "node_summary".to_string(),
            PromptTemplateCategoryEnum::EdgeSummary => "edge_summary".to_string(),
            PromptTemplateCategoryEnum::CustomQuestion => "custom_question".to_string(),
        }
    }
}

pub trait LlmContext {
    fn get_context(&self) -> Self;
    fn render_prompt(&self, prompt_template: &str) -> String;
}

impl LlmContext for Entity {
    fn get_context(&self) -> Self {
        self.clone()
    }

    fn render_prompt(&self, prompt_template: &str) -> String {
        let mut prompt = prompt_template.to_string();
        prompt = prompt.replace("{{entity_name}}", &self.name);
        prompt = prompt.replace("{{entity_id}}", &self.id);
        prompt = prompt.replace("{{entity_type}}", &self.label);
        prompt
    }
}

impl LlmContext for ExpandedRelation {
    fn get_context(&self) -> Self {
        self.clone()
    }

    fn render_prompt(&self, prompt_template: &str) -> String {
        let mut prompt = prompt_template.to_string();
        prompt = prompt.replace("{{source_name}}", &self.source.name);
        prompt = prompt.replace("{{source_id}}", &self.source.id);
        prompt = prompt.replace("{{source_type}}", &self.source.label);
        prompt = prompt.replace("{{relation_type}}", &self.relation.relation_type);
        prompt = prompt.replace("{{target_name}}", &self.target.name);
        prompt = prompt.replace("{{target_id}}", &self.target.id);
        prompt = prompt.replace("{{target_type}}", &self.target.label);
        prompt
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::FromRow, Object, Validate)]
pub struct LlmMessage<
    T: LlmContext
        + Send
        + Sync
        + poem_openapi::types::Type
        + poem_openapi::types::ParseFromJSON
        + poem_openapi::types::ToJSON,
> {
    #[serde(skip_deserializing)]
    #[oai(read_only)]
    pub id: i32,

    #[validate(regex(
        path = "UUID_REGEX",
        message = "The session_id must match the ^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$ pattern."
    ))]
    pub session_uuid: String,
    pub prompt_template: String,
    pub prompt_template_category: PromptTemplateCategory,

    pub context: T,
    pub prompt: String,
    pub message: String,

    #[serde(skip_deserializing)]
    #[serde(with = "ts_seconds")]
    #[oai(read_only)]
    pub created_at: DateTime<Utc>,

    #[serde(skip_deserializing)]
    #[serde(with = "ts_seconds")]
    #[oai(read_only)]
    pub updated_at: DateTime<Utc>,
}

impl<T: LlmContext> LlmMessage<T>
where
    T: LlmContext
        + Send
        + Sync
        + poem_openapi::types::Type
        + poem_openapi::types::ParseFromJSON
        + poem_openapi::types::ToJSON,
{
    pub fn new(
        prompt_template_category: &str,
        context: T,
        session_uuid: Option<String>,
    ) -> Result<Self, anyhow::Error> {
        let session_uuid = match session_uuid {
            Some(session_uuid) => session_uuid,
            None => uuid::Uuid::new_v4().to_string(),
        };

        let prompt_template = match PROMPT_TEMPLATE.get(prompt_template_category) {
            Some(prompt_template) => prompt_template.to_string(),
            None => return Err(anyhow::anyhow!("Invalid prompt template category")),
        };

        let prompt = context.render_prompt(prompt_template.as_str());
        let message = "".to_string();

        let prompt_template_category = PromptTemplateCategory::from(prompt_template_category.to_string());

        Ok(LlmMessage {
            id: 0,
            session_uuid,
            prompt_template,
            prompt_template_category,
            context,
            prompt,
            message,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub async fn save2db(&self, pool: &sqlx::PgPool) -> Result<&Self, anyhow::Error> {
        let sql_str = "INSERT INTO biomedgps_ai_message (session_uuid, prompt_template, prompt_template_category, context, prompt, message, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id";

        let prompt_template_category: String = self.prompt_template_category.clone().into();
        let context: String = self.context.to_json_string();
        let query = sqlx::query(&sql_str)
            .bind(&self.session_uuid)
            .bind(&self.prompt_template)
            .bind(&prompt_template_category)
            .bind(&context)
            .bind(&self.prompt)
            .bind(&self.message)
            .bind(&self.created_at)
            .bind(&self.updated_at);

        match query.execute(pool).await {
            Ok(_) => return Ok(self),
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to save message to database: {}",
                    e.to_string()
                ))
            }
        };
    }

    pub async fn answer(&mut self, chatbot: &ChatBot, pool: Option<&sqlx::PgPool>) -> Result<&Self, anyhow::Error> {
        let prompt = self.prompt.clone();
        self.message = chatbot.answer(prompt)?;
        self.updated_at = Utc::now();

        if pool.is_none() {
            return Ok(self);
        }

        match self.save2db(pool.unwrap()).await {
            Ok(_) => {
                Ok(self)
            }
            Err(e) => Err(anyhow::anyhow!(
                "Failed to save message to database: {}",
                e.to_string()
            )),
        }
    }
}

pub struct ChatBot {
    role: MessageRole,
    name: Option<String>,
    content: Option<String>,
    function_call: Option<FunctionCall>,
    model_name: String,
    client: Client,
}

impl ChatBot {
    pub fn new(model_name: &str, openai_api_key: &str) -> Self {
        let model = if model_name == "GPT4" {
            GPT4.to_string()
        } else {
            GPT3_5_TURBO.to_string()
        };

        let client = Client::new(openai_api_key.to_string());

        ChatBot {
            role: MessageRole::user,
            name: None,
            content: None,
            function_call: None,
            model_name: model,
            client: client,
        }
    }

    pub fn answer(&self, prompt: String) -> Result<String, anyhow::Error> {
        let model_name = self.model_name.clone();
        let req = ChatCompletionRequest::new(
            model_name,
            vec![chat_completion::ChatCompletionMessage {
                role: self.role.clone(),
                content: prompt,
                name: self.name.clone(),
                function_call: self.function_call.clone(),
            }],
        );

        let result = self.client.chat_completion(req)?;
        let message = result.choices[0].message.content.clone();

        match message {
            Some(message) => Ok(message),
            None => Err(anyhow::anyhow!("No message returned")),
        }
    }
}


// Write unit tests
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_answer() {
        let OPENAI_API_KEY = std::env::var("OPENAI_API_KEY").unwrap();
        let chatbot = super::ChatBot::new("GPT3", &OPENAI_API_KEY);

        let node = super::Entity {
            idx: 0,
            id: "DrugBank:DB01050".to_string(),
            name: "IBUPROFEN".to_string(),
            label: "Compound".to_string(),
            resource: "DrugBank".to_string(),
            description: None,
            taxid: None,
            synonyms: None,
            pmids: None,
            xrefs: None,
        };

        let mut llm_msg = super::LlmMessage::new("node_summary", node, None).unwrap();
        let answer = llm_msg.answer(&chatbot, None).await.unwrap();
        println!("Prompt: {}", answer.prompt);
        println!("Answer: {}", answer.message);

        assert_eq!(answer.message.len() > 0, true);
    }
}