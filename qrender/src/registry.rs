use crate::{
    custom::dimension::DimensionsRenderer,
    error::QRenderError,
    rendering::{DefaultRenderer, Renderer},
};

pub struct RendererRegistry {}

impl RendererRegistry {
    pub fn get_renderer(name: &str) -> Result<Box<dyn Renderer>, QRenderError> {
        match name {
            "default" => Ok(Box::new(DefaultRenderer)),
            "dimensions" => Ok(Box::new(DimensionsRenderer)),
            _ => Err(QRenderError::UnknownRenderer(name.to_string())),
        }
    }
}
