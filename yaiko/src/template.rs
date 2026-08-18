use crate::{Request, Response};
use handlebars::Handlebars;
use serde::Serialize;
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
    pub dev_mode_dir: Option<String>,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine {
    pub fn new() -> Self {
        TemplateEngine {
            handlebars: Handlebars::new(),
            dev_mode_dir: None,
        }
    }

    pub fn new_with_dir(dir_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let mut engine = Self::new();
        engine.load_dir(dir_path)?;

        if std::env::var("APP_ENV").unwrap_or_default() == "development" {
            engine.dev_mode_dir = Some(dir_path.to_string());
        }

        Ok(engine)
    }

    fn load_dir(&mut self, dir_path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "html" || ext == "hbs" {
                        // Use relative path without extension as the template name
                        if let Ok(rel_path) = path.strip_prefix(dir_path) {
                            let name = rel_path.with_extension("").to_string_lossy().to_string();
                            self.handlebars.register_template_file(&name, path)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn register_template_file(
        &mut self,
        name: &str,
        path: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.handlebars.register_template_file(name, path)?;
        Ok(())
    }

    pub fn render<T: Serialize>(
        &mut self,
        req: &Request,
        template: &str,
        data: &T,
    ) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let reload_dir = self.dev_mode_dir.clone();
        if let Some(dir) = reload_dir {
            self.handlebars.clear_templates();
            self.load_dir(&dir)?;
        }

        let mut json_data = serde_json::to_value(data)?;

        // Auto-inject CSRF token from cookie if present
        if let Some(obj) = json_data.as_object_mut() {
            if let Some(cookie_str) = req.headers.get("cookie").and_then(|h| h.to_str().ok()) {
                if let Some(token) = cookie_str.split(';').find_map(|c| {
                    let mut parts = c.trim().splitn(2, '=');
                    if parts.next() == Some("csrf_token") {
                        parts.next()
                    } else {
                        None
                    }
                }) {
                    obj.insert(
                        "csrf_token".to_string(),
                        serde_json::Value::String(token.to_string()),
                    );
                }
            }
        }

        let rendered = self.handlebars.render(template, &json_data)?;
        Ok(Response::new().html(&rendered))
    }
}
