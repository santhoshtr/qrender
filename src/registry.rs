use crate::{
    custom::dimension::DimensionsRenderer,
    rendering::{DefaultRenderer, Renderer},
};

pub struct RendererRegistry {}

impl RendererRegistry {
    pub fn get_renderer(name: &str) -> Box<dyn Renderer> {
        match name {
            "default" => Box::new(DefaultRenderer),
            "dimensions" => Box::new(DimensionsRenderer),
            _ => panic!("Renderer not found"),
        }
    }
}
