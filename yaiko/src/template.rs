use crate::Response;
use handlebars::Handlebars;
use serde::Serialize;

#[derive(Clone, Debug)]  // Added Clone trait here
pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        TemplateEngine {
            handlebars: Handlebars::new(),
        }
    }

    pub fn register_template_file(&mut self, name: &str, path: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.handlebars.register_template_file(name, path)?;
        Ok(())
    }

    pub fn render<T: Serialize>(&self, template: &str, data: &T) -> Result<Response, Box<dyn std::error::Error + Send + Sync>> {
        let rendered = self.handlebars.render(template, data)?;
        Ok(Response::new().html(&rendered))
    }
}