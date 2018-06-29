use std::sync::Arc;
use std::sync::Mutex;
use std::collections::VecDeque;

use winit::Window;
use winit::EventsLoop;
use winit::WindowBuilder;

mod ui_state;
use self::ui_state::{ UIState };

mod input_state;
pub use self::input_state::{ InputState };

mod access;
pub use self::access::{
    WorldAccess,
    ModelAccess,
    WindowAccess,
    RenderAccess,
    RenderLayerAccess,
    InputAccess,
};

mod render_state;
pub use self::render_state::{
    DrawMode,
    SceneGraph,
    RenderState,
    WindowWithEvents,
};

use super::Renderer;
use super::model::Model;

use thing;
use thing::World;

///
/// This is the central, and global, state passed to each mod during the main loop
///
pub struct State {

    /// Root container of the Thing/Facet system (game world state)
    world: World,

    /// Container for all rendering state
    render_state: RenderState,

    /// Container for all input state
    input_state: InputState,

    /// Container for all UI related state
    ui_state: UIState,
}

impl State {
    /// Create a new, empty State object
    pub fn new() -> Self {
        State{
            world: World::new(),
            render_state: RenderState::new(),
            input_state: InputState {
                pending_input_events: VecDeque::new(),
                other_input_sources: Vec::new() // input sources added at runtime
            },
            ui_state: UIState::new(),
        }
    }
}
