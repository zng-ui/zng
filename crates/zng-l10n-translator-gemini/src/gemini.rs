use std::{error::Error, fmt, time::Duration};

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

#[derive(Deserialize, Debug)]
struct Candidate {
    pub content: Content,
    #[serde(rename = "finishReason")]
    pub finish_reason: String,
}

#[derive(Deserialize, Debug)]
struct GeminiResponse {
    pub candidates: Vec<Candidate>,
}

#[derive(Deserialize, Debug)]
struct GeminiErrorResponse {
    pub error: GeminiError,
}
#[derive(Deserialize, Debug)]
struct GeminiError {
    pub code: u16,
    pub message: String,
}

#[derive(Serialize, Debug)]
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

    let mut retries = 0;
    let mut gemini_response = loop {
        let uri = Uri::try_from(format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
        ))?;
        let request = Request::new(Method::POST, uri)
            .header("x-goog-api-key", &api_key)?
            .body_json(&GeminiRequest {
                system_instruction: Content {
                    parts: vec![Part {
                        text: system_prompt.clone(),
                    }],
                },
                contents: vec![Content {
                    parts: vec![Part { text: input.clone() }],
                }],
            })?;

        let mut response = send(request).await?;
        match response.body_json::<GeminiResponse>().await {
            Ok(r) => break r,
            Err(_) => {
                if let Ok(gemini_error) = response.body_json::<GeminiErrorResponse>().await {
                    if gemini_error.error.code == 429 {
                        // too many requests
                        if retries < 5 {
                            retries += 1;
                            zng_task::deadline(Duration::from_mins(1)).await;
                            continue;
                        }
                    }
                    return Err(e(gemini_error.error));
                }
                let other_error = response.body_text().await.unwrap_or_default();
                return Err(e(UnknownGeminiError(other_error.into())));
            }
        }
    };

    // Just check if model completed response, Fluent syntax validation is done by cargo-zng
    if let Some(mut r) = gemini_response.candidates.pop()
        && r.finish_reason == "STOP"
        && let Some(p) = r.content.parts.pop()
    {
        Ok(p.text)
    } else {
        Err(e(InvalidResponse(gemini_response)))
    }
}
fn e(e: impl Error + Send + Sync + 'static) -> Box<dyn Error + Send + Sync> {
    Box::new(e)
}

#[derive(Debug)]
struct InvalidResponse(GeminiResponse);
impl fmt::Display for InvalidResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid response\n{:#?}", self.0)
    }
}
impl std::error::Error for InvalidResponse {}
impl fmt::Display for GeminiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}
impl std::error::Error for GeminiError {}

#[derive(Debug)]
struct UnknownGeminiError(String);
impl fmt::Display for UnknownGeminiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown response:\n{}", self.0)
    }
}
impl std::error::Error for UnknownGeminiError {}
