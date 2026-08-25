
use common_game::components::resource as resource_lib;
use common_game::components::resource::{ResourceType, BasicResource, BasicResourceType, ComplexResource, ComplexResourceType, GenericResource};
use explorer_common::Bag;
pub struct Inventory {
    bag: Bag,
}

impl Inventory {
    pub fn new(bag: Bag) -> Inventory {
        Inventory { bag }
    }

    pub fn get_bag(&mut self) -> &mut Bag {
        &mut self.bag
    }

    pub fn has_resource(&self, resource_type: common_game::components::resource::ResourceType) -> usize {
        self.bag.contains(resource_type)
    }

    pub fn get_basic(&mut self, what: BasicResourceType) -> Option<GenericResource> {
        let mut ret = None;
        for i in 0..self.bag.resources.len().clone() {
            match self.bag.resources[i].get_type() {
                ResourceType::Basic(A) => {
                    if ret.is_some() {
                        ret = Some(self.bag.resources.remove(i));
                    }
                }
                _ => { ret = None}
            }
        }
        ret
    }
    pub fn get_complex(&mut self, what: ComplexResourceType) -> Option<GenericResource> {
        let mut ret = None;
        for i in 0..self.bag.resources.len().clone() {
            match self.bag.resources[i].get_type() {
                ResourceType::Complex(B) => {
                    if ret.is_none() && what == B {
                        ret = Some(self.bag.resources.remove(i));
                    }
                }
                _ => { ret = None }
            }
        }
        ret
    }
}