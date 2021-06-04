use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CompletionRequest {
    pub prompt: String,
    pub max_tokens: i32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]

pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Choice {
    pub text: String,
    pub index: i32,
    pub finish_reason: String,
}

pub struct Client {
    pub token: String,
}

impl Client {
    pub fn new(token: String) -> Self {
        Client { token }
    }

    pub async fn completion(
        &self,
        req: &CompletionRequest,
    ) -> Result<CompletionResponse, reqwest::Error> {
        let res: CompletionResponse = reqwest::Client::new()
            .post("https://api.openai.com/v1/engines/davinci/completions")
            .bearer_auth(&self.token)
            .json(req)
            .send()
            .await?
            .json()
            .await?;
        Ok(res)
    }
}
