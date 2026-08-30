pub mod udev;
pub mod winit;

use smithay::{
    backend::session::Session,
    desktop::{Space, Window},
    output::Output,
    reexports::calloop::EventLoop,
};

use udev::UdevData;
use winit::WinitData;

use crate::state::{enki::Enki, State};

pub enum RenderResult {
    Submitted,
    NoDamage,
    Skipped,
}

pub enum Backend {
    Winit(WinitData),
    Udev(UdevData),
}

impl Backend {
    pub fn init(&mut self, event_loop: &EventLoop<State>, enki: &mut Enki) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            Backend::Winit(w) => w.init(enki),
            Backend::Udev(u) => u.init(event_loop, enki),
        }
    }

    pub fn seat_name(&self) -> String {
        match self {
            Backend::Winit(_) => "winit".to_string(),
            Backend::Udev(u) => u.session.seat(),
        }
    }

    pub fn render_frame(&mut self, space: &Space<Window>) -> RenderResult {
        match self {
            Backend::Winit(w) => w.render(space),
            Backend::Udev(_) => RenderResult::Skipped,
        }
    }

    pub fn event_loop_tick(&mut self, enki: &Enki) {
        match self {
            Backend::Winit(_) => {}
            Backend::Udev(u) => u.render_all(enki),
        }
    }

    pub fn winit(&mut self) -> &mut WinitData {
        match self {
            Self::Winit(w) => w,
            _ => panic!(),
        }
    }

    pub fn udev(&mut self) -> &mut UdevData {
        match self {
            Self::Udev(u) => u,
            _ => panic!(),
        }
    }
}
