mod communication;
mod movement;
mod items;
mod decisionmaking;

const SLEEP_MILLIS: u64 = 2000;

use std::cell::RefCell;
use std::rc::Rc;
use std::thread::sleep;
use std::time::Duration;

use crate::communication::{OrchestratorComms, PlanetComms};
use crate::items::Inventory;
use crate::movement::Memory;
use crate::movement::Thrusters;
use crate::decisionmaking::Brain;

use explorer_common::{Bag, BagContent, Explorer as ExplorerTrait, AiReturn};

use common_game::utils::ID;
use crossbeam_channel::{Receiver, Sender};

pub struct Explorer {
    id: ID,
    is_auto: bool,
    brain: Brain,
    orchestrator_comms: Rc<RefCell<OrchestratorComms>>,
    planet_comms: Rc<RefCell<PlanetComms>>,
    inventory: Inventory,
    thrusters: Thrusters,
}

pub enum InterruptOrder {
    None,
    Reset,
    Stop,
    Die
}

impl PartialEq for InterruptOrder {
    fn eq(&self, other: &Self) -> bool {
        matches!((self, other),
            (InterruptOrder::None, InterruptOrder::None) |
            (InterruptOrder::Reset, InterruptOrder::Reset) |
            (InterruptOrder::Stop, InterruptOrder::Stop) |
            (InterruptOrder::Die, InterruptOrder::Die)
        )
    }
}

impl  ExplorerTrait for Explorer {
    fn new(
        id: ID,
        bag: Bag,
        digit_planet_id: ID,
        planet_channel: explorer_common::logged_channel::LoggedChannel<common_game::protocols::planet_explorer::ExplorerToPlanet, common_game::protocols::planet_explorer::PlanetToExplorer>,
        orchestrator_channel: explorer_common::logged_channel::LoggedChannel::<common_game::protocols::orchestrator_explorer::ExplorerToOrchestrator<BagContent>, common_game::protocols::orchestrator_explorer::OrchestratorToExplorer>
    ) -> Self {
        log::debug!("Explorer: Calling 'New' on LazyBoone");

        let planet_id = Rc::new(RefCell::new(digit_planet_id));
        let inventory = Inventory::new(bag);
        let planet_comms = Rc::new(RefCell::new(PlanetComms::new(id, planet_channel)));
        let orchestrator_comms = Rc::new(RefCell::new(OrchestratorComms::new(id, planet_id.clone(), orchestrator_channel)));
        let memory =  Memory::new(planet_id.clone(), orchestrator_comms.clone(), planet_comms.clone());

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
    fn explorer_ai(&mut self) -> AiReturn {
        log::debug!("LazyBoone: Exploring starting planet");
        match self.thrusters.memory.explore() {
            Ok(()) => {/*Proceed to loop*/}
            Err(InterruptOrder::None) => {panic!("Initial exploration before loop somehow had a non-interruption error")},
            Err(InterruptOrder::Reset) => {return AiReturn::Reset},
            Err(InterruptOrder::Stop) => {return AiReturn::Stop},
            Err(InterruptOrder::Die) => {return AiReturn::Kill}
        }; //Guarantee to know where you are now
        loop {
            log::debug!("LazyBoone: Explorer_AI ran for a tick");
            self.brain.populate_plans();
            let order = self.brain.solve_best_plan(&mut self.thrusters, &mut self.inventory);
            match order {
                Ok(()) => {
                    //Just in case, better poll the orchestrator in case the best plan was idling (which doesn't include comms)
                    match self.orchestrator_comms.borrow().poll() {
                        Ok(()) => {/*Proceed to loop*/}
                        Err(InterruptOrder::None) => {panic!("Polling encountered some unknown error")},
                        Err(InterruptOrder::Reset) => {return AiReturn::Reset},
                        Err(InterruptOrder::Stop) => {return AiReturn::Stop},
                        Err(InterruptOrder::Die) => {return AiReturn::Kill}
                    };
                }
                Err(InterruptOrder::None) => {},
                Err(InterruptOrder::Reset) => {
                    log::debug!("LazyBoone: Resetting command propagated at head");
                    return AiReturn::Reset
                },
                Err(InterruptOrder::Stop) => {
                    log::debug!("LazyBoone: Stopping command propagated at head");
                    return AiReturn::Stop
                },
                Err(InterruptOrder::Die) => {
                    log::debug!("LazyBoone: Kill command propagated at head");
                    return AiReturn::Kill
                }
            }
            sleep(Duration::from_millis(SLEEP_MILLIS));
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
        self.thrusters.memory.override_current_id(id);
    }

    fn get_auto_mode(&self) -> bool {
        self.is_auto
    }

    fn set_auto_mode(&mut self, auto_mode: bool) {
        self.is_auto = auto_mode;
    }

    fn reset(&mut self) {
        self.brain.reset();
        self.thrusters.memory.forget_all();
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
