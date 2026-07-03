use handlebars::Handlebars;
use serde_json::json;

use crate::error::QRenderError;
use crate::model::Property;

pub trait Renderer {
    fn render_text(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, QRenderError>;
    fn render_markdown(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, QRenderError>;
    fn render_wikitext(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, QRenderError>;
    fn render_html(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, QRenderError>;
}

// Default Renderer
pub struct DefaultRenderer;

fn render_with_template(
    template: &str,
    group_name: &str,
    properties: &[Property],
) -> Result<String, QRenderError> {
    let mut handlebars = Handlebars::new();
    handlebars
        .register_template_string("template", template)
        .map_err(Box::new)?;
    let data = json!({
        "group_name": group_name,
        "properties": properties,
    });
    Ok(handlebars.render("template", &data)?)
}

impl Renderer for DefaultRenderer {
    fn render_text(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, QRenderError> {
        render_with_template(
            include_str!("../templates/default.text.hbs"),
            group_name,
            properties,
        )
    }

    fn render_markdown(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, QRenderError> {
        render_with_template(
            include_str!("../templates/default.markdown.hbs"),
            group_name,
            properties,
        )
    }

    fn render_wikitext(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, QRenderError> {
        render_with_template(
            include_str!("../templates/default.wikitext.hbs"),
            group_name,
            properties,
        )
    }

    fn render_html(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, QRenderError> {
        render_with_template(
            include_str!("../templates/default.html.hbs"),
            group_name,
            properties,
        )
    }
}
