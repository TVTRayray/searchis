use crate::{
    error::AppError,
    model::{CreateSnippetInput, Snippet, UpdateSnippetInput},
    storage::Repository,
    validation::{validate_create, validate_update},
};

pub struct SnippetService {
    repository: Repository,
}

impl SnippetService {
    pub fn new(repository: Repository) -> Self {
        Self { repository }
    }

    pub fn create(&self, input: CreateSnippetInput) -> Result<Snippet, AppError> {
        let fields = validate_create(&input)?;
        self.repository.create(fields, &input.request_id)
    }

    pub fn get(&self, id: &str) -> Result<Snippet, AppError> {
        self.repository
            .get(id)?
            .ok_or_else(|| AppError::new("NOT_FOUND", "片段不存在。请重新载入列表。"))
    }

    pub fn list(&self) -> Result<Vec<Snippet>, AppError> {
        self.repository.list()
    }

    pub fn update(&self, input: UpdateSnippetInput) -> Result<Snippet, AppError> {
        let fields = validate_update(&input)?;
        self.repository.update(&input.id, input.revision, fields)
    }
}
