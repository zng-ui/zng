use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};
use zng_ext_l10n::Lang;

#[derive(Serialize, Deserialize, Debug)]
struct Part {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Content {
    pub parts: Vec<Part>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Candidate {
    pub content: Content,
    #[serde(rename = "finishReason")]
    pub finish_reason: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiResponse {
    pub candidates: Vec<Candidate>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiRequest {
    pub contents: Vec<Content>,
    pub system_instruction: Content,
}

pub async fn translate(
    api_key: String,
    model: String,
    from_lang: Lang,
    to_lang: Lang,
    input: String,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    fn lang_name(l: Lang) -> String {
        let code = format!("{l:?}");
        if l.autonym().is_some() {
            return format!("{code} ({l:#})");
        }
        code
    }
    let system_prompt = format!(
        "Translate this Fluent file text from `{}` to `{}`. Only output the translated file text",
        lang_name(from_lang),
        lang_name(to_lang)
    );

    if std::env::var("GEMINI_TRANSLATOR_TEST").is_ok() {
        return Ok(format!(
            r"
### GEMINI_TRANSLATOR_TEST enabled
### prompt: {system_prompt}

{input}"
        ));
    }

    use zng_task::http::*;

    let uri = Uri::try_from(format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
    ))?;
    let request = Request::new(Method::POST, uri)
        .header("x-goog-api-key", api_key)?
        .body_json(&GeminiRequest {
            system_instruction: Content {
                parts: vec![Part { text: system_prompt }],
            },
            contents: vec![Content {
                parts: vec![Part { text: input }],
            }],
        })?;
    let mut response = send(request).await?.body_json::<GeminiResponse>().await?;

    // Just check if model completed response, Fluent syntax validation is done by cargo-zng
    if let Some(mut r) = response.candidates.pop()
        && r.finish_reason == "STOP"
        && let Some(p) = r.content.parts.pop()
    {
        Ok(p.text)
    } else {
        struct InvalidResponse(GeminiResponse);
        impl fmt::Debug for InvalidResponse {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_tuple("InvalidResponse").field(&self.0).finish()
            }
        }
        impl fmt::Display for InvalidResponse {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "invalid response\n{:#?}", self.0)
            }
        }
        impl std::error::Error for InvalidResponse {}
        Err(Box::new(InvalidResponse(response)))
    }
}
