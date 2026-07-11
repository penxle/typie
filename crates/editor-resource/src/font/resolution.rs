#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub family_id: u16,
    pub weight: u16,
    pub chunk_id: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Ready(Target),
    Pending { target: Target, needs_base: bool },
    AwaitingManifest { family_id: u16, weight: u16 },
    Missing,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construct_variants() {
        let t = Target {
            family_id: 1,
            weight: 400,
            chunk_id: 0,
        };
        let _ = Resolution::Ready(t);
        let _ = Resolution::Pending {
            target: t,
            needs_base: false,
        };
        let _ = Resolution::Pending {
            target: t,
            needs_base: true,
        };
        let _ = Resolution::Missing;
    }
}
