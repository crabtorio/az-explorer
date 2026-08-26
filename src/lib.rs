mod communication;
mod movement;
mod items;
mod decisionmaking;

use std::cell::RefCell;
use std::rc::Rc;
use crate::communication::{OrchestratorComms, PlanetComms};
use crate::items::Inventory;
use crate::movement::Memory;
use crate::movement::Thrusters;
use crate::decisionmaking::Brain;

use explorer_common::{Bag, BagContent, Explorer};

use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

struct LazyBoone {
    id: ID,
    is_auto: bool,
    brain: Brain,
    orchestrator_comms: Rc<RefCell<OrchestratorComms>>,
    planet_comms: Rc<RefCell<PlanetComms>>,
    inventory: Inventory,
    thrusters: Thrusters
}
impl Explorer for LazyBoone {
    fn new(
        id: ID,
        bag: Bag,
        planet_id: ID,
        planet_channel: explorer_common::logged_channel::LoggedChannel<common_game::protocols::planet_explorer::ExplorerToPlanet, common_game::protocols::planet_explorer::PlanetToExplorer>,
        orchestrator_channel: explorer_common::logged_channel::LoggedChannel::<common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator<BagContent>, common_game::protocols::orchestrator_explorer::OrchestratorToExplorer>
    ) -> Self {

        let inventory = Inventory::new(bag);
        let planet_comms = Rc::new(RefCell::new(PlanetComms::new(id, planet_id, planet_channel)));
        let orchestrator_comms = Rc::new(RefCell::new(OrchestratorComms::new(id, planet_id, orchestrator_channel)));
        let memory =  Memory::new(planet_id, orchestrator_comms.clone(), planet_comms.clone());

        Self {
            id,
            is_auto: false,
            brain: Brain::new(planet_comms.clone()),
            orchestrator_comms: orchestrator_comms.clone(),
            planet_comms: planet_comms.clone(),
            thrusters: Thrusters::new(memory, orchestrator_comms.clone()),
            inventory
        }
    }
    fn run(&mut self) {
        log::info!("LazyBoone: Run");
        loop {
            self.try_recv_from_orchestrator_and_respond();

            if self.is_auto {
                self.brain.populate_plans();
                self.brain.solve_best_plan(&mut self.thrusters, &mut self.inventory);
            }
        }
    }

    fn get_id(&self) -> ID {
        self.id
    }

    fn get_bag(&mut self) -> &mut Bag {
        self.inventory.get_bag()
    }

    fn get_planet_id(& self) -> ID {
        self.thrusters.memory.get_current_id()
    }

    fn set_planet_id(&mut self, id: ID) {
        if self.thrusters.move_adj(id).is_ok() {
            self.thrusters.memory.override_current_id(id);
            self.planet_comms.borrow_mut().override_current_id(id);
            self.orchestrator_comms.borrow_mut().override_current_id(id);
        }
    }

    fn get_auto_mode(&self) -> bool {
        self.is_auto
    }

    fn set_auto_mode(&mut self, auto_mode: bool) {
        self.is_auto = auto_mode;
        if !auto_mode {
            self.brain.clear_plans();
        }
    }

    fn get_planet_channel(&self) -> explorer_common::logged_channel::LoggedChannel<common_game::protocols::planet_explorer::ExplorerToPlanet, common_game::protocols::planet_explorer::PlanetToExplorer> {
        self.planet_comms.borrow_mut().get_channel()
    }

    fn set_planet_channel_tx(&mut self, tx: Sender<common_game::protocols::planet_explorer::ExplorerToPlanet>) {
        self.planet_comms.borrow_mut().set_tx(tx);
    }

    fn set_planet_channel_rx(&mut self, rx: Receiver<common_game::protocols::planet_explorer::PlanetToExplorer>) {
        self.planet_comms.borrow_mut().set_rx(rx);
    }

    fn get_orchestrator_channel(&self) -> explorer_common::logged_channel::LoggedChannel<common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator<BagContent>, common_game::protocols::orchestrator_explorer::OrchestratorToExplorer> {
        self.orchestrator_comms.borrow_mut().get_channel()
    }

    fn set_orchestrator_channel_tx(&mut self, tx: Sender<common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator<BagContent>>) {
        self.orchestrator_comms.borrow_mut().set_tx(tx);
    }

    fn set_orchestrator_channel_rx(&mut self, rx: Receiver<common_game::protocols::orchestrator_explorer::OrchestratorToExplorer>) {
        self.orchestrator_comms.borrow_mut().set_rx(rx);
    }


}
