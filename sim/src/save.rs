//! Saving and restoring a game in progress (the whole `World`; rules come from the data).

use crate::rules::Rules;
use crate::world::World;

/// Bump when the layout of the saved types changes, so old saves are discarded instead of
/// misread.
const FORMAT: u32 = 7;
const MAGIC: &[u8; 4] = b"VNDT";

impl World {
    pub fn save(&self) -> Vec<u8> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend(FORMAT.to_le_bytes());
        postcard::to_extend(self, bytes).expect("serialising the world")
    }

    /// Restores a save made by [`World::save`], checking it matches these rules.
    pub fn load(rules: &Rules, bytes: &[u8]) -> Result<World, String> {
        let body = bytes
            .strip_prefix(MAGIC.as_slice())
            .and_then(|b| b.strip_prefix(FORMAT.to_le_bytes().as_slice()))
            .ok_or("not a save of this version")?;
        let world: World = postcard::from_bytes(body).map_err(|e| format!("corrupt save: {e}"))?;
        let consistent = world.persos.iter().all(|p| p.race < rules.races.len())
            && world.fiefs.iter().all(|f| f.people < rules.peoples.len())
            && world.houses.iter().all(|h| h.kind < rules.buildings.len());
        if !consistent {
            return Err("save made with other game data".into());
        }
        world.prepare_paths(rules);
        Ok(world)
    }
}
