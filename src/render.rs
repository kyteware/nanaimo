use smithay::{
    backend::renderer::{
        damage::{Error as OutputDamageTrackerError, OutputDamageTracker, RenderOutputResult},
        element::{
            surface::WaylandSurfaceRenderElement,
            Wrap,
        },
        Color32F, ImportAll, ImportMem, Renderer,
    },
    desktop::space::{
        Space, SpaceRenderElements,
    },
    output::Output,
    desktop::Window,
};

// Removed unused import: use crate::state::NanaimoState;

smithay::backend::renderer::element::render_elements! {
    pub CustomRenderElements<R> where
        R: ImportAll + ImportMem;
    Surface=WaylandSurfaceRenderElement<R>,
}

smithay::backend::renderer::element::render_elements! {
    pub OutputRenderElements<R, E> where R: ImportAll + ImportMem;
    Space=SpaceRenderElements<R, E>,
    Window=Wrap<E>,
    Custom=CustomRenderElements<R>,
}

pub fn render_output<'a, 'd, R>(
    output: &'a Output,
    space: &'a Space<Window>,
    renderer: &'a mut R,
    framebuffer: &'a mut R::Framebuffer<'_>,
    damage_tracker: &'d mut OutputDamageTracker,
    age: usize,
) -> Result<RenderOutputResult<'d>, OutputDamageTrackerError<R::Error>>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Clone + 'static,
{
    let elements = smithay::desktop::space::space_render_elements::<_, Window, _>(
        renderer,
        [space],
        output,
        1.0,
    )
    .expect("output without mode?");
    
    let output_render_elements = elements.into_iter().map(OutputRenderElements::Space).collect::<Vec<_>>();

    let clear_color = Color32F::new(0.1, 0.1, 0.1, 1.0);

    damage_tracker.render_output(renderer, framebuffer, age, &output_render_elements, clear_color)
}
