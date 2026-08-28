
use common_game::components::resource::{ResourceType, BasicResourceType, ComplexResourceType, GenericResource};
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

    pub fn put_in_bag(&mut self, resource: GenericResource) {
        log::debug!{"Lazyboone: Putting a {:?} away for a sunny day", resource}

        self.bag.resources.push(resource);
    }

    pub fn has_resource(&self, resource_type: ResourceType) -> usize {
        self.bag.contains(resource_type)
    }

    pub fn get_basic(&mut self, what: BasicResourceType) -> Option<GenericResource> {
        log::trace!("LazyBoone: Pulling a {:?} from his pockets", what);
        let mut ret = None;
        for i in 0..self.bag.resources.len() {
            match self.bag.resources[i].get_type() {
                ResourceType::Basic(a) => {
                    if ret.is_none() && a == what {
                        ret = Some(self.bag.resources.remove(i));
                    }
                }
                _ => { ret = None}
            }
        }
        ret
    }
    pub fn get_complex(&mut self, what: ComplexResourceType) -> Option<GenericResource> {
        log::trace!("LazyBoone: Pulling a {:?} from his backpack", what);
        let mut ret = None;
        for i in 0..self.bag.resources.len() {
            match self.bag.resources[i].get_type() {
                ResourceType::Complex(b) => {
                    if ret.is_none() && what == b {
                        ret = Some(self.bag.resources.remove(i));
                    }
                }
                _ => { ret = None }
            }
        }
        ret
    }
}