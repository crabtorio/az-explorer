mod communication;
mod movement;
mod items;
mod decisionmaking;

use crate::communication::{OrchestratorComms, PlanetComms};
use crate::items::Inventory;

use explorer_common::Explorer;

struct LazyBoone {
    orchestrator_comms: OrchestratorComms,
    planet_comms: PlanetComms,
    inventory: Inventory,
}

impl Explorer for LazyBoone {

}
