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

pub async fn translate(
    url: String,
    model: String,
    from_lang: Lang,
    to_lang: Lang,
    input: String,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    use zng_task::http::*;

    let system_prompt = r#"You are a professional localization engineer and translator.

CRITICAL SYNTAX RULES:
1. Preserve ALL Fluent syntax structure exactly as-is:
   - Message keys and nesting.
   - Attribute names (e.g., `.label`, `.tooltip`).
   - Variables and interpolations (e.g., `{{ $name }}`, `{{ $count }}`).
   - Selectors and match arms (e.g., `->`, `[one]`, `*[other]`).
   - Term definitions and references (e.g., `-brand`, `-app-name`).
2. Only translate the human-readable text values and variant strings. Do not alter keys, variable identifiers, or syntax keywords.
3. Maintain exact indentation, spacing, and line breaks.
4. Output ONLY the translated Fluent file inside a markdown code block. No introductory text, no explanations."#;

    fn lang_name(l: Lang) -> String {
        let code = format!("{l:?}");
        if l.autonym().is_some() {
            return format!("{code} ({l:#})");
        }
        code
    }
    let from = lang_name(from_lang);
    let to = lang_name(to_lang);
    let user_prompt = format!("Translate the following Fluent (.ftl) file from `{from}` to `{to}`.\n\n```ftl\n{input}\n```");

    if std::env::var("LOCAL_TRANSLATOR_LLM_TEST").is_ok() {
        let mut test = "### LOCAL_TRANSLATOR_LLM_TEST enabled\n\n### <|system|>\n".to_owned();
        for line in system_prompt.lines() {
            test.push_str("### ");
            test.push_str(line);
            test.push('\n');
        }
        test.push_str("\n### <|user|>\n");
        for line in user_prompt.lines() {
            test.push_str("### ");
            test.push_str(line);
            test.push('\n');
        }
        return Ok(test);
    }

    let url = Uri::try_from(url)?;

    let request = Request::new(Method::POST, url.clone()).body_json(&OpenAiRequest {
        model,
        messages: vec![
            Message {
                role: "system".to_owned(),
                content: system_prompt.to_owned(),
            },
            Message {
                role: "user".to_owned(),
                content: user_prompt,
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
