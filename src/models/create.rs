use serde::{Deserialize, Serialize};

use crate::Ollama;

/// A stream of `CreateModelStatus` objects
#[cfg(feature = "stream")]
pub type CreateModelStatusStream =
    std::pin::Pin<Box<dyn tokio_stream::Stream<Item = crate::error::Result<CreateModelStatus>>>>;

impl Ollama {
    #[cfg(feature = "stream")]
    /// Create a model with streaming, meaning that each new status will be streamed.
    pub async fn create_model_stream(
        &self,
        model_name: String,
        path: String,
    ) -> crate::error::Result<CreateModelStatusStream> {
        use tokio_stream::StreamExt;

        use crate::error::OllamaError;

        let request = CreateModelRequest {
            model_name,
            path,
            stream: true,
        };

        let uri = format!("{}/api/create", self.uri());
        let serialized = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        let res = self
            .reqwest_client
            .post(uri)
            .body(serialized)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Err(res.text().await.unwrap_or_else(|e| e.to_string()).into());
        }

        let stream = Box::new(res.bytes_stream().map(|res| match res {
            Ok(bytes) => {
                let res = serde_json::from_slice::<CreateModelStatus>(&bytes);
                match res {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        let err = serde_json::from_slice::<crate::error::OllamaError>(&bytes);
                        match err {
                            Ok(err) => Err(err),
                            Err(_) => Err(OllamaError::from(format!(
                                "Failed to deserialize response: {}",
                                e
                            ))),
                        }
                    }
                }
            }
            Err(e) => Err(OllamaError::from(format!("Failed to read response: {}", e))),
        }));

        Ok(std::pin::Pin::from(stream))
    }

    /// Create a model with a single response, only the final status will be returned.
    pub async fn create_model(
        &self,
        model_name: String,
        path: String,
    ) -> crate::error::Result<CreateModelStatus> {
        let request = CreateModelRequest {
            model_name,
            path,
            stream: false,
        };

        let uri = format!("{}/api/create", self.uri());
        let serialized = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        let res = self
            .reqwest_client
            .post(uri)
            .body(serialized)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !res.status().is_success() {
            return Err(res.text().await.unwrap_or_else(|e| e.to_string()).into());
        }

        let res = res.bytes().await.map_err(|e| e.to_string())?;
        let res = serde_json::from_slice::<CreateModelStatus>(&res).map_err(|e| e.to_string())?;

        Ok(res)
    }
}

/// A create model request to Ollama.
#[derive(Serialize)]
struct CreateModelRequest {
    #[serde(rename = "name")]
    model_name: String,
    path: String,
    stream: bool,
}

/// A create model status response from Ollama.
#[derive(Deserialize, Debug)]
pub struct CreateModelStatus {
    #[serde(rename = "status")]
    pub message: String,
}
