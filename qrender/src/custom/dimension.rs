use handlebars::Handlebars;
use serde_json::json;

use crate::{error::QRenderError, model::Property, rendering::Renderer};

// Custom Renderer for Dimensions
pub struct DimensionsRenderer;

fn render_plain(group_name: &str, properties: &[Property]) -> Result<String, QRenderError> {
    let mut text = String::new();
    text.push_str("Dimensions Group:\n");
    for property in properties {
        for statement in &property.statements {
            text.push_str(&format!(
                "{}\n  {}: {} \n",
                group_name, property.wd_label, statement.value
            ));
        }
    }
    Ok(text)
}

impl Renderer for DimensionsRenderer {
    fn render_text(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, QRenderError> {
        render_plain(group_name, properties)
    }

    fn render_markdown(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, QRenderError> {
        render_plain(group_name, properties)
    }

    fn render_wikitext(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, QRenderError> {
        render_plain(group_name, properties)
    }

    fn render_html(
        &self,
        group_name: &str,
        properties: &[Property],
    ) -> Result<String, QRenderError> {
        let mut handlebars = Handlebars::new();
        let template_string = r#"
        <div>
            <h1>{{group_name}}</h1>
            <ul>
            {{#each properties}}
                <li>{{wd_label}}: {{statements.0.value}}</li>
            {{/each}}
            </ul>
            </div>
        "#;
        handlebars
            .register_template_string("dimensions_template", template_string)
            .map_err(Box::new)?;

        let data = &json!({
            "group_name": group_name,
            "properties": properties,
        });
        Ok(handlebars.render("dimensions_template", &data)?)
    }
}
