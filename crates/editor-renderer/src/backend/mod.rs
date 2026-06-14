pub mod cpu;

use crate::sink::RenderSink;
use cpu::CpuSink;

pub enum RenderBackend {
    Cpu(CpuSink),
}

impl RenderBackend {
    pub fn new_cpu(width: u16, height: u16) -> Self {
        Self::Cpu(CpuSink::new(width, height))
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        match self {
            Self::Cpu(s) => s.resize(width, height),
        }
    }

    pub fn render(&mut self, f: impl FnOnce(&mut dyn RenderSink)) {
        match self {
            Self::Cpu(s) => f(s),
        }
    }

    pub fn sink(&mut self) -> &mut dyn RenderSink {
        match self {
            Self::Cpu(s) => s,
        }
    }

    pub fn take_touched(&mut self) -> bool {
        match self {
            Self::Cpu(s) => s.take_touched(),
        }
    }
}
