use skrifa::instance::{NormalizedCoord, Size};
use skrifa::outline::{
    Engine, HintingInstance, HintingOptions, OutlineGlyphCollection, OutlineGlyphFormat,
    SmoothMode, Target,
};

const MAX_CACHED_HINT_INSTANCES: usize = 8;

const HINTING_OPTIONS: HintingOptions = HintingOptions {
    engine: Engine::AutoFallback,
    target: Target::Smooth {
        mode: SmoothMode::Lcd,
        symmetric_rendering: true,
        preserve_linear_metrics: true,
    },
};

pub struct HintingCache {
    glyf_entries: Vec<HintingEntry>,
    cff_entries: Vec<HintingEntry>,
    serial: u64,
}

impl HintingCache {
    pub fn new() -> Self {
        Self {
            glyf_entries: Vec::new(),
            cff_entries: Vec::new(),
            serial: 0,
        }
    }

    pub fn get(
        &mut self,
        id: [u64; 2],
        outlines: &OutlineGlyphCollection<'_>,
        size: Size,
        coords: &[NormalizedCoord],
    ) -> Option<&HintingInstance> {
        let entries = match outlines.format()? {
            OutlineGlyphFormat::Glyf => &mut self.glyf_entries,
            OutlineGlyphFormat::Cff | OutlineGlyphFormat::Cff2 => &mut self.cff_entries,
        };

        let (entry_ix, is_current) = find_entry(entries, id, outlines, size, coords)?;
        let entry = entries.get_mut(entry_ix)?;

        self.serial += 1;
        entry.serial = self.serial;

        if !is_current {
            entry.id = id;
            entry
                .instance
                .reconfigure(outlines, size, coords, HINTING_OPTIONS)
                .ok()?;
        }
        Some(&entry.instance)
    }
}

struct HintingEntry {
    id: [u64; 2],
    instance: HintingInstance,
    serial: u64,
}

fn find_entry(
    entries: &mut Vec<HintingEntry>,
    id: [u64; 2],
    outlines: &OutlineGlyphCollection<'_>,
    size: Size,
    coords: &[NormalizedCoord],
) -> Option<(usize, bool)> {
    let mut found_serial = u64::MAX;
    let mut found_index = 0;

    for (ix, entry) in entries.iter().enumerate() {
        if entry.id == id
            && entry.instance.size() == size
            && entry.instance.location().coords() == coords
        {
            return Some((ix, true));
        }

        if entry.serial < found_serial {
            found_serial = entry.serial;
            found_index = ix;
        }
    }

    if entries.len() < MAX_CACHED_HINT_INSTANCES {
        let instance = HintingInstance::new(outlines, size, coords, HINTING_OPTIONS).ok()?;
        let ix = entries.len();
        entries.push(HintingEntry {
            id,
            instance,
            serial: 0,
        });

        Some((ix, true))
    } else {
        Some((found_index, false))
    }
}
