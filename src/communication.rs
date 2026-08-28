use std::collections::HashSet;
use std::{cell::RefCell, rc::Rc};

use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResource, ComplexResourceRequest, ComplexResourceType, ResourceType};
use common_game::components::resource::BasicResourceType::{Carbon, Hydrogen, Oxygen};
use common_game::components::resource::ComplexResourceType::{Diamond, Life, Robot, Water};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;

use crossbeam_channel::{Receiver, Sender};

use explorer_common::BagContent;
use explorer_common::logged_channel::LoggedChannel;
use crate::InterruptOrder;
use crate::movement::AnyResource;
use crate::items::Inventory;

pub struct OrchestratorComms {
    id: ID,
    current: Rc<RefCell<ID>>,
    channel: LoggedChannel<ExplorerToOrchestrator<BagContent>, OrchestratorToExplorer>,
}

pub struct PlanetComms {
    id: ID,
    channel: LoggedChannel<ExplorerToPlanet, PlanetToExplorer>,
}

impl OrchestratorComms {
    pub fn new(id: ID, current: Rc<RefCell<ID>>,channel: LoggedChannel<ExplorerToOrchestrator<BagContent>, OrchestratorToExplorer>) -> OrchestratorComms {
        log::trace!("LazyBoone: Creating OrchestratorComms");
        Self {
            id,
            current,
            channel,
        }
    }

    pub fn get_channel(&mut self) -> LoggedChannel<ExplorerToOrchestrator<BagContent>, OrchestratorToExplorer> {
        self.channel.clone()
    }

    pub fn set_rx(&mut self, rx: Receiver<OrchestratorToExplorer>) {
        self.channel.set_receiver(rx);
    }

    pub fn set_tx(&mut self, tx: Sender<ExplorerToOrchestrator<BagContent>>) {
        self.channel.set_sender(tx);
    }
    pub fn get_adjs(&self) -> Result<Vec<ID>, InterruptOrder> {
        log::trace!("LazyBoone: Requesting adjs from orchestrator");
        let now = *self.current.borrow();
        if self.channel.send(ExplorerToOrchestrator::NeighborsRequest {explorer_id: self.id, current_planet_id: now }).is_ok() {
            match self.channel.recv() {
                Err(_) => {
                    panic!("Something went wrong when receiving planet neighbours request");
                }
                Ok(something) => {
                    match something {
                        OrchestratorToExplorer::NeighborsResponse{neighbors} => {
                            Ok(neighbors)
                        }
                        OrchestratorToExplorer::ResetExplorerAI => {
                            log::trace!("LazyBoone: Received unexpected reset, but keeping calm");
                            Err(InterruptOrder::Reset)
                        },
                        OrchestratorToExplorer::StopExplorerAI => {
                            log::trace!("LazyBoone: Received unexpected stop, but keeping calm");
                            Err(InterruptOrder::Stop)
                        }
                        OrchestratorToExplorer::KillExplorer => {
                            log::trace!("LazyBoone: Received unexpected kill, but keeping calm");
                            Err(InterruptOrder::Die)
                        },
                        _ => {panic!("Orchestrator sent something wrong")}
                    }
                }
            }
        } else {
            panic!("Can't send the explorer request");
        }
    }

    pub fn request_move(&self, dest: ID) -> Result<(Sender<ExplorerToPlanet>, ID), InterruptOrder> {
        log::trace!("LazyBoone: Requesting motion");
        let now = *self.current.borrow();
        if self.channel.send(ExplorerToOrchestrator::TravelToPlanetRequest {explorer_id: self.id, current_planet_id: now, dst_planet_id: dest}).is_ok() {
            match self.channel.recv() {
                Ok(OrchestratorToExplorer::MoveToPlanet {sender_to_new_planet: a, planet_id: b}) => {
                    if let Some(innie) = a {
                        if self.channel.send(ExplorerToOrchestrator::MovedToPlanetResult {explorer_id:self.id, planet_id: dest}).is_ok() {
                            Ok((innie, b))
                        } else {
                            panic!("Unable to send movement ack");
                        }

                    } else {
                        Err(InterruptOrder::None)
                    }
                },
                Ok(OrchestratorToExplorer::ResetExplorerAI) => {
                    log::trace!("LazyBoone: Received unexpected reset, but keeping calm");
                    Err(InterruptOrder::Reset)
                },
                Ok(OrchestratorToExplorer::StopExplorerAI) => {
                    log::trace!("LazyBoone: Received unexpected stop, but keeping calm");
                    Err(InterruptOrder::Stop)
                },
                Ok(OrchestratorToExplorer::KillExplorer) => {
                    log::trace!("LazyBoone: Received unexpected kill, but keeping calm");
                    Err(InterruptOrder::Die)
                },
                _ => {panic!("Wrong reply received");}
            }
        } else {
            panic!("Can't send the explorer move request");
        }
    }

    pub fn poll(&self) -> Result<(), InterruptOrder> {
        log::trace!("LazyBoone: Making sure perseverance is appreciated");

        match self.channel.poll() {
            Ok(None) => {
                //Go ahead
                Ok(())
            },
            Ok(Some(OrchestratorToExplorer::KillExplorer)) => {
                Err(InterruptOrder::Die)
            },
            Ok(Some(OrchestratorToExplorer::ResetExplorerAI)) => {
                Err(InterruptOrder::Reset)
            },
            Ok(Some(OrchestratorToExplorer::StopExplorerAI)) => {
                Err(InterruptOrder::Stop)
            },
            Ok(Some(_)) => {
                panic!("Unexpected request");
            }
            Err(_) => {
                panic!("Could not poll Orchestrator");
            }
        }

    }
}

impl PlanetComms {
    pub fn new(id: ID, channel: LoggedChannel<ExplorerToPlanet, PlanetToExplorer>) -> PlanetComms {
        log::trace!("LazyBoone: Creating PlanetComms");
        Self {
            id,
            channel,
        }
    }
    pub fn get_channel(&mut self) -> LoggedChannel<ExplorerToPlanet, PlanetToExplorer> {
        self.channel.clone()
    }

    pub fn set_rx(&mut self, rx: Receiver<PlanetToExplorer>) {
        self.channel.set_receiver(rx);
    }

    pub fn set_tx(&mut self, tx: Sender<ExplorerToPlanet>) {
        self.channel.set_sender(tx);
    }
    pub fn get_prods(&self) -> HashSet<AnyResource> {
        log::trace!("LazyBoone: Requesting productions from planet");
        let mut result: HashSet<AnyResource> = HashSet::new();
        if self.channel.send(ExplorerToPlanet::SupportedResourceRequest{ explorer_id: self.id }).is_ok() {
            if let Ok(PlanetToExplorer::SupportedResourceResponse {resource_list}) = self.channel.recv() {
                if self.channel.send(ExplorerToPlanet::SupportedCombinationRequest { explorer_id: self.id}).is_ok() {
                    if let Ok(PlanetToExplorer::SupportedCombinationResponse {combination_list}) = self.channel.recv() {
                        for i in combination_list.iter() {
                            result.insert(AnyResource::from(i));
                        }
                        for i in resource_list.iter() {
                            result.insert(AnyResource::from(i));
                        }
                        result
                    } else {
                        panic!("Something went wrong when receiving combination request");
                    }
                } else {
                    panic!("Can't send the explorer combination recipe request");
                }
            } else {
                panic!("Something went wrong when receiving planet basic resource request");
            }
        } else {
            panic!("Can't send the explorer generation recipe request");
        }
    }

    pub fn try_get(&self, what: BasicResourceType) -> Option<BasicResource>  {
        log::debug!("LazyBoone: Requesting basic resource ({:?}) from planet", what);
        if self.channel.send(ExplorerToPlanet::GenerateResourceRequest {explorer_id: self.id, resource: what}).is_ok() {
            if let Ok(PlanetToExplorer::GenerateResourceResponse {resource}) = self.channel.recv() {
                resource
            } else {
                panic!("Something went wrong when receiving resource request");
            }
        } else {
            panic!("Can't send the explorer generation request");
        }
    }

    pub fn try_craft(&self, bag: &mut Inventory, what: ComplexResourceType) -> Option<ComplexResource> {
        log::debug!("LazyBoone: Requesting complex resource ({:?}) from planet", what);
        let request = match what {
            ComplexResourceType::AIPartner => {
                if bag.has_resource(ResourceType::Complex(Robot)) > 0 && bag.has_resource(ResourceType::Complex(Diamond)) > 0 {
                    let (Some(a), Some(b)) = (bag.get_complex(Robot), bag.get_complex(Diamond)) else {panic!("Bag changed between resource check and grab")};
                    ComplexResourceRequest::AIPartner(a.to_robot().expect("Item changed identity between grab and use"), b.to_diamond().expect("Item changed identity between grab and use"))
                } else {
                    return None;
                }
            }
            ComplexResourceType::Diamond => {
                if bag.has_resource(ResourceType::Basic(Carbon)) > 1 {
                    let (Some(a), Some(b)) = (bag.get_basic(Carbon), bag.get_basic(Carbon))else {panic!("Bag changed between resource check and grab")};
                    ComplexResourceRequest::Diamond(a.to_carbon().expect("Item changed identity between grab and use"), b.to_carbon().expect("Item changed identity between grab and use"))
                } else {
                    return None;
                }
            }
            ComplexResourceType::Dolphin => {
                if bag.has_resource(ResourceType::Complex(Life)) > 0 && bag.has_resource(ResourceType::Complex(Water)) > 0 {
                    let (Some(a), Some(b)) = (bag.get_complex(Water), bag.get_complex(Life))else {panic!("Bag changed between resource check and grab")};
                    ComplexResourceRequest::Dolphin(a.to_water().expect("Item changed identity between grab and use"), b.to_life().expect("Item changed identity between grab and use"))
                } else {
                    return None;
                }
            }
            ComplexResourceType::Robot => {
                if bag.has_resource(ResourceType::Basic(BasicResourceType::Silicon)) > 0 && bag.has_resource(ResourceType::Complex(Life)) > 0 {
                    let (Some(a), Some(b)) = (bag.get_basic(BasicResourceType::Silicon), bag.get_complex(Life)) else {panic!("Bag changed between resource check and grab")};
                    ComplexResourceRequest::Robot(a.to_silicon().expect("Item changed identity between grab and use"), b.to_life().expect("Item changed identity between grab and use"))
                } else {
                    return None;
                }
            }
            ComplexResourceType::Life => {
                if bag.has_resource(ResourceType::Complex(Water)) > 0 && bag.has_resource(ResourceType::Basic(Carbon)) > 0 {
                    let (Some(a), Some(b)) = (bag.get_complex(Water), bag.get_basic(Carbon)) else {panic!("Bag changed between resource check and grab")};
                    ComplexResourceRequest::Life(a.to_water().expect("Item changed identity between grab and use"), b.to_carbon().expect("Item changed identity between grab and use"))
                } else {
                    return None;
                }
            }
            ComplexResourceType::Water => {
                if bag.has_resource(ResourceType::Basic(Hydrogen)) > 0 && bag.has_resource(ResourceType::Basic(Oxygen)) > 0 {
                    let (Some(a), Some(b)) = (bag.get_basic(Hydrogen), bag.get_basic(Oxygen)) else {panic!("Bag changed between resource check and grab")};
                    ComplexResourceRequest::Water(a.to_hydrogen().expect("Item changed identity between grab and use"), b.to_oxygen().expect("Item changed identity between grab and use"))
                } else {
                    return None;
                }
            }
        };
        if self.channel.send(ExplorerToPlanet::CombineResourceRequest {explorer_id: self.id, msg: request}).is_ok() {
            if let Ok(PlanetToExplorer::CombineResourceResponse {complex_response}) = self.channel.recv() {
                complex_response.ok()
            } else {
                panic!("Something went wrong when receiving resource craft");
            }
        } else {
            panic!("Can't send the explorer generation recipe request");
        }
    }
}