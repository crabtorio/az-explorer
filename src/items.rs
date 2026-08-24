/* TODO
- Import Resources
- Import communication to planet
- Create Bag
- Implement bag analysis and scoring
- Implement bag management
 */

/*
use common_game::components::resource as resource_lib;
use resource_lib::ResourceType as ResourceIdeaContainer;
use resource_lib::BasicResourceType as BasicResourceIdea;
use resource_lib::ComplexResourceType as ComplexResourceIdea;
*/

use explorer_common::Bag;
use crate::communication::PlanetComms;

pub struct Inventory {
    bag: Bag,
}

impl Inventory {
    pub fn new() -> Inventory {
        Inventory { bag: Bag::new() }
    }

    fn request_item() {

    }

    fn try_craft() {

    }
}