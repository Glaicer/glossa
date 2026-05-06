use async_trait::async_trait;

use crate::AppError;

/// Text enhancement abstraction for post-processing STT output.
#[async_trait]
pub trait TextEnhancer: Send + Sync {
    fn name(&self) -> &'static str;

    async fn enhance(&self, text: &str) -> Result<String, AppError>;
}

/// No-op enhancer that returns text unchanged.
#[derive(Debug, Clone, Copy)]
pub struct NoopTextEnhancer;

#[async_trait]
impl TextEnhancer for NoopTextEnhancer {
    fn name(&self) -> &'static str {
        "noop"
    }

    async fn enhance(&self, text: &str) -> Result<String, AppError> {
        Ok(text.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_enhancer_should_return_text_unchanged() {
        let enhancer = NoopTextEnhancer;
        let result = enhancer
            .enhance("hello world")
            .await
            .expect("should succeed");
        assert_eq!(result, "hello world");
    }

    #[tokio::test]
    async fn noop_enhancer_name_should_be_noop() {
        assert_eq!(NoopTextEnhancer.name(), "noop");
    }
}
