use handlebars::Handlebars;
use serde_json::json;

use crate::{model::Property, rendering::Renderer};

// Custom Renderer for Dimensions
pub struct DimensionsRenderer;

impl Renderer for DimensionsRenderer {
    fn render_text(
        &self,
        group_name: &str,
        properties: &Vec<Property>,
    ) -> Result<String, Box<dyn std::error::Error>> {
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

    fn render_markdown(
        &self,
        group_name: &str,
        properties: &Vec<Property>,
    ) -> Result<String, Box<dyn std::error::Error>> {
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
    fn render_wikitext(
        &self,
        group_name: &str,
        properties: &Vec<Property>,
    ) -> Result<String, Box<dyn std::error::Error>> {
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

    fn render_html(
        &self,
        group_name: &str,
        properties: &Vec<Property>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        //Implement handlebar rendering logic here
        let mut handlebars = Handlebars::new();
        // Define the template.  You can also load this from a file.
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

        // Register the template
        handlebars.register_template_string("dimensions_template", template_string)?;

        let data = &json!({
            "group_name": group_name,
            "properties": properties.iter().map(|(property)| {
                json!({
                    "pid": property.pid,
                    "wd_label": property.wd_label,
                    "statements": property.statements
                })
            }).collect::<Vec<_>>()
        });
        let rendering = handlebars.render("dimensions_template", &data)?;
        Ok(rendering)
    }
}
