use std::collections::HashSet;
use std::rc::Rc;
use std::cell::RefCell;

use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResource, ComplexResourceRequest, ComplexResourceType, ResourceType};
use common_game::components::resource::BasicResourceType::{Carbon, Hydrogen, Oxygen};
use common_game::components::resource::ComplexResourceType::{Diamond, Life, Robot, Water};
use common_game::protocols::orchestrator_explorer::{ExplorerToOrchestrator, OrchestratorToExplorer};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
use common_game::utils::ID;

use crossbeam_channel::{Receiver, Sender, SendError};

use explorer_common::BagContent;
use explorer_common::logged_channel::{LoggedChannel, ChannelError};

use crate::movement::AnyResource;
use crate::items::Inventory;
use crate::movement::AnyResource::Silicon;

pub struct OrchestratorComms {
    id: ID,
    current: ID,
    channel: LoggedChannel<ExplorerToOrchestrator<BagContent>, OrchestratorToExplorer>,
}

pub struct PlanetComms {
    id: ID,
    current: ID,
    channel: LoggedChannel<ExplorerToPlanet, PlanetToExplorer>,
}

impl OrchestratorComms {
    pub fn new(id: ID, current: ID,channel: LoggedChannel<ExplorerToOrchestrator<BagContent>, OrchestratorToExplorer>) -> OrchestratorComms {
        Self {
            id,
            current,
            channel,
        }
    }
    
    pub fn override_current_id(&mut self, id: ID) {
        self.id = id;
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
    pub fn get_adjs(&self) -> Vec<ID> {
        if self.channel.send(ExplorerToOrchestrator::NeighborsRequest {explorer_id: self.id, current_planet_id: self.current }).is_ok() {
            match self.channel.recv() {
                Err(RecvError) => {
                    panic!("Something went wrong when receiving planet neighbours request");
                }
                Ok(Something) => {
                    match Something {
                        OrchestratorToExplorer::NeighborsResponse{neighbors} => {
                            neighbors
                        }
                        _ => {panic!("Orchestrator sent something wrong")}
                    }
                }
            }
        } else {
            panic!("Can't send the explorer request");
        }
    }

    pub fn request_move(&self, dest: ID) -> Result<(), SendError<ExplorerToOrchestrator<BagContent>>> {
        self.channel.send(ExplorerToOrchestrator::TravelToPlanetRequest {explorer_id: self.id, current_planet_id: self.current, dst_planet_id: dest})
    }
}

impl PlanetComms {
    pub fn new(id: ID, current:ID, channel: LoggedChannel<ExplorerToPlanet, PlanetToExplorer>) -> PlanetComms {
        Self {
            id,
            current,
            channel,
        }
    }

    pub fn override_current_id(&mut self, id: ID) {
        self.id = id;
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
        let request = match what {
            ComplexResourceType::AIPartner => {
                if bag.has_resource(ResourceType::Complex(Robot)) > 0 && bag.has_resource(ResourceType::Complex(Diamond)) > 0 {
                    let (Some(A), Some(B)) = (bag.get_complex(Robot), bag.get_complex(Diamond))else {panic!("This shouldn't happen")};
                    ComplexResourceRequest::AIPartner{0:A.to_robot().unwrap(), 1:B.to_diamond().unwrap()}
                } else {
                    return None;
                }
            }
            ComplexResourceType::Diamond => {
                if bag.has_resource(ResourceType::Basic(Carbon)) > 1 {
                    let (Some(A), Some(B)) = (bag.get_basic(Carbon), bag.get_basic(Carbon))else {panic!("This shouldn't happen")};
                    ComplexResourceRequest::Diamond{0:A.to_carbon().unwrap(), 1:B.to_carbon().unwrap()}
                } else {
                    return None;
                }
            }
            ComplexResourceType::Dolphin => {
                if bag.has_resource(ResourceType::Complex(Life)) > 0 && bag.has_resource(ResourceType::Complex(Water)) > 0 {
                    let (Some(A), Some(B)) = (bag.get_complex(Water), bag.get_complex(Life))else {panic!("This shouldn't happen")};
                    ComplexResourceRequest::Dolphin{0:A.to_water().unwrap(), 1:B.to_life().unwrap()}
                } else {
                    return None;
                }
            }
            ComplexResourceType::Robot => {
                if bag.has_resource(ResourceType::Basic(BasicResourceType::Silicon)) > 0 && bag.has_resource(ResourceType::Complex(Life)) > 0 {
                    let (Some(A), Some(B)) = (bag.get_basic(BasicResourceType::Silicon), bag.get_complex(Life)) else {panic!("This shouldn't happen")};
                    ComplexResourceRequest::Robot{0:A.to_silicon().unwrap(), 1:B.to_life().unwrap()}
                } else {
                    return None;
                }
            }
            ComplexResourceType::Life => {
                if bag.has_resource(ResourceType::Complex(Water)) > 0 && bag.has_resource(ResourceType::Basic(Carbon)) > 0 {
                    let (Some(A), Some(B)) = (bag.get_complex(Water), bag.get_basic(Carbon)) else {panic!("This shouldn't happen")};
                    ComplexResourceRequest::Life{0:A.to_water().unwrap(), 1:B.to_carbon().unwrap()}
                } else {
                    return None;
                }
            }
            ComplexResourceType::Water => {
                if bag.has_resource(ResourceType::Basic(Hydrogen)) > 0 && bag.has_resource(ResourceType::Basic(Oxygen)) > 0 {
                    let (Some(A), Some(B)) = (bag.get_basic(Hydrogen), bag.get_basic(Oxygen)) else {panic!("This shouldn't happen")};
                    ComplexResourceRequest::Water{0:A.to_hydrogen().unwrap(), 1:B.to_oxygen().unwrap()}
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