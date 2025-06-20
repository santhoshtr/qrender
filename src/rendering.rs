use handlebars::Handlebars;
use serde_json::json;

use crate::model::Property;

pub trait Renderer {
    fn render_text(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, Box<dyn std::error::Error>>;
    fn render_markdown(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, Box<dyn std::error::Error>>;
    fn render_wikitext(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, Box<dyn std::error::Error>>;
    fn render_html(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, Box<dyn std::error::Error>>;
}

// Default Renderer
pub struct DefaultRenderer;

impl Renderer for DefaultRenderer {
    fn render_text(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("template", include_str!("../templates/default.text.hbs"))
            .unwrap();
        let data = json!({
            "group_name": group_name,
            "properties": properties,
        });
        let rendered = handlebars.render("template", &data)?;
        Ok(rendered)
    }
    fn render_markdown(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string(
                "template-md",
                include_str!("../templates/default.markdown.hbs"),
            )
            .unwrap();
        let data = json!({
            "group_name": group_name,
            "properties": properties,
        });
        let rendered = handlebars.render("template-md", &data)?;
        Ok(rendered)
    }
    fn render_wikitext(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string(
                "template-wikitext",
                include_str!("../templates/default.wikitext.hbs"),
            )
            .unwrap();
        let data = json!({
            "group_name": group_name,
            "properties": properties,
        });
        let rendered = handlebars.render("template-wikitext", &data)?;
        Ok(rendered)
    }

    fn render_html(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut handlebars = Handlebars::new();
        handlebars
            .register_template_string("template", include_str!("../templates/default.html.hbs"))
            .unwrap();
        let data = json!({
            "group_name": group_name,
            "properties": properties,
        });
        let rendered = handlebars.render("template", &data)?;
        Ok(rendered)
    }
}
