use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};
use zng_ext_l10n::Lang;

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize, Debug)]
struct Choice {
    pub message: Message,
    #[allow(unused)]
    pub finish_reason: String,
}

#[derive(Deserialize, Debug)]
struct OpenAiResponse {
    pub choices: Vec<Choice>,
}

#[derive(Serialize, Debug)]
struct OpenAiRequest {
    pub model: String,
    pub messages: Vec<Message>,
}

pub async fn translate(url: String, from_lang: Lang, to_lang: Lang, input: String) -> Result<String, Box<dyn Error + Send + Sync>> {
    use zng_task::http::*;

    fn lang_name(l: Lang) -> String {
        let code = format!("{l:?}");
        if l.autonym().is_some() {
            return format!("{code} ({l:#})");
        }
        code
    }
    let system_prompt = format!(
        "You are a professional localization translator. Translate the following Fluent file from `{}` to `{}`. Preserve all Fluent syntax exactly as-is: message keys, attributes (`.label`, `.tooltip`), variables (`{{ $name }}`), selectors, and terms (`-brand`). Only translate the human-readable text values. Use natural, idiomatic language for the target locale. Output the translated Fluent file in a markdown code block.",
        lang_name(from_lang),
        lang_name(to_lang)
    );

    if std::env::var("LOCAL_TRANSLATOR_LLM_TEST").is_ok() {
        return Ok(format!(
            r"
### LOCAL_TRANSLATOR_LLM_TEST enabled
### prompt: {system_prompt}

{input}"
        ));
    }

    let url = Uri::try_from(url)?;

    let request = Request::new(Method::POST, url.clone()).body_json(&OpenAiRequest {
        model: "llama".to_owned(),
        messages: vec![
            Message {
                role: "system".to_owned(),
                content: system_prompt.to_owned(),
            },
            Message {
                role: "user".to_owned(),
                content: input.to_owned(),
            },
        ],
    })?;

    let mut response = send(request).await?;
    response.error().await?;
    let r = response.body_json::<OpenAiResponse>().await?;

    if let Some(r) = r.choices.first() {
        let r = &r.message.content;
        let mut start = "";
        let mut end = "";
        for line in r.lines().rev() {
            if end.is_empty() {
                if line == "```" {
                    end = line;
                }
            } else if start.is_empty() && line.starts_with("```") {
                start = line;
                break;
            }
        }
        if !start.is_empty() && !end.is_empty() {
            let r_ptr = r.as_ptr() as usize;
            let start = (start.as_ptr() as usize + start.len() + "\n".len()) - r_ptr;
            let end = (end.as_ptr() as usize - "\n".len()) - r_ptr;
            let r = &r[start..end];
            if !r.trim().is_empty() {
                return Ok(r.to_owned());
            }
        }
    }
    Err(Box::new(InvalidResponse(r)))
}

#[derive(Debug)]
struct InvalidResponse(OpenAiResponse);
impl fmt::Display for InvalidResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid response\n{:#?}", self.0)
    }
}
impl std::error::Error for InvalidResponse {}
