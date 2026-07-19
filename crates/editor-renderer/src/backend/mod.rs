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

    pub fn try_new_cpu(width: u16, height: u16) -> Option<Self> {
        CpuSink::try_new(width, height).map(Self::Cpu)
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        match self {
            Self::Cpu(s) => s.resize(width, height),
        }
    }

    pub fn try_resize(&mut self, width: u16, height: u16) -> bool {
        match self {
            Self::Cpu(s) => s.try_resize(width, height),
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

    pub fn cpu_sink(&mut self) -> &mut CpuSink {
        match self {
            Self::Cpu(s) => s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_sink_returns_concrete_cpu_sink_with_matching_dims() {
        let mut backend = RenderBackend::new_cpu(10, 10);
        let sink = backend.cpu_sink();
        assert_eq!(sink.pixel_size(), (10, 10));
    }

    #[test]
    fn try_new_cpu_returns_backend_with_matching_dims() {
        let mut backend = RenderBackend::try_new_cpu(10, 10).expect("small alloc should succeed");
        assert_eq!(backend.cpu_sink().pixel_size(), (10, 10));
    }

    #[test]
    fn try_resize_backend_updates_dims_on_success() {
        let mut backend = RenderBackend::new_cpu(4, 4);
        assert!(backend.try_resize(8, 8));
        assert_eq!(backend.cpu_sink().pixel_size(), (8, 8));
    }
}
