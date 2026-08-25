use std::collections::{HashMap, VecDeque};
use std::ops::{Add, AddAssign};
use common_game::components::resource::{BasicResourceType, ComplexResourceType, ResourceType};
use common_game::components::resource::ResourceType::Complex;
use crate::movement::{AnyResource, Memory, EXPECTED_RANDOM_COST};

const MOVEMENT_COST: i32 = -1;
const EXPLORATION_COST: i32 = EXPECTED_RANDOM_COST as i32 * MOVEMENT_COST;
const ITEM_MULT: i32 = 5;
const BASIC_VALUE: i32 = 1 * ITEM_MULT;
const COMPLEXITY_OFFSET: i32 = 1 * ITEM_MULT;
const WATER_VALUE: i32 = BASIC_VALUE * 2 + COMPLEXITY_OFFSET;
const DIAMOND_VALUE: i32 = BASIC_VALUE * 2 + COMPLEXITY_OFFSET;
const LIFE_VALUE: i32 = WATER_VALUE + BASIC_VALUE + COMPLEXITY_OFFSET;
const DOLPHIN_VALUE: i32 = LIFE_VALUE + BASIC_VALUE + COMPLEXITY_OFFSET;
const ROBOT_VALUE: i32 = LIFE_VALUE + BASIC_VALUE + COMPLEXITY_OFFSET;
const AIPARTNER_VALUE: i32 = ROBOT_VALUE + DIAMOND_VALUE + COMPLEXITY_OFFSET;

struct Score {s: i32}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        self.s += rhs.s;
    }
}

pub struct Expectedscore {s: i32}

impl Expectedscore {

    pub fn new(s: i32) -> Expectedscore {
        Expectedscore{s}
    }
    fn to_score(self) -> Score {
        Score{s: self.s}
    }
}

impl Add for Expectedscore {
    type Output = Expectedscore;

    fn add(self, rhs: Self) -> Self::Output {
        Expectedscore{s: self.s + rhs.s}
    }
}


type ActionResult = Result<(), ()>;

enum Actions {
    Grab(BasicResourceType), //insert item type here, should create it's move command on creation, using the weights
    Craft(ComplexResourceType), //insert item type here, like grab should create move
}

impl Actions {
    fn get_expected_score(&self) -> Expectedscore {
        match self {
            Actions::Grab(_) => {Expectedscore::new(BASIC_VALUE)},
            Actions::Craft(thing) => {
                match thing {
                    ComplexResourceType::Water => {Expectedscore::new(WATER_VALUE)},
                    ComplexResourceType::Diamond => {Expectedscore::new(DIAMOND_VALUE)},
                    ComplexResourceType::Life => {Expectedscore::new(LIFE_VALUE)},
                    ComplexResourceType:: Dolphin => {Expectedscore::new(DOLPHIN_VALUE)},
                    ComplexResourceType::Robot => {Expectedscore::new(ROBOT_VALUE)},
                    ComplexResourceType::AIPartner => {Expectedscore::new(AIPARTNER_VALUE)},
                }
            }
        }
    }

    fn get_resource(&self) -> AnyResource {
        match self {
            Actions::Grab(thing) => {
                AnyResource::from(thing)
            }
            Actions::Craft(thing) => {
                AnyResource::from(thing)
            }
        }
    }
}

impl Clone for Actions {
    fn clone(&self) -> Self {
        match self {
            Actions::Grab(resource) => Actions::Grab(resource.clone()),
            Actions::Craft(resource) => Actions::Craft(resource.clone()),
        }
    }
}

struct Plan {
    prerequisites: HashMap<ResourceType, usize>,
    action: Actions,
}

impl Plan {
    fn new(what: ResourceType) -> Plan {

        let mut prerequisites: HashMap<ResourceType, usize> = HashMap::new();
        match what {
            ResourceType::Basic(_) => {},
            ResourceType::Complex(ComplexResourceType::Water) => {
                prerequisites.insert(ResourceType::Basic(BasicResourceType::Oxygen), 1);
                prerequisites.insert(ResourceType::Basic(BasicResourceType::Hydrogen), 1);
            }
            ResourceType::Complex(ComplexResourceType::Diamond) => {
                prerequisites.insert(ResourceType::Basic(BasicResourceType::Carbon), 2);
            }
            ResourceType::Complex(ComplexResourceType::Life) => {
                prerequisites.insert(ResourceType::Basic(BasicResourceType::Carbon), 1);
                prerequisites.insert(ResourceType::Complex(ComplexResourceType::Water), 1);
            }
            ResourceType::Complex(ComplexResourceType::Dolphin) => {
                prerequisites.insert(ResourceType::Complex(ComplexResourceType::Water), 1);
                prerequisites.insert(ResourceType::Complex(ComplexResourceType::Life), 1);
            }
            ResourceType::Complex(ComplexResourceType::Robot) => {
                prerequisites.insert(ResourceType::Complex(ComplexResourceType::Life), 1);
                prerequisites.insert(ResourceType::Basic(BasicResourceType::Silicon), 1);
            }
            ResourceType::Complex(ComplexResourceType::AIPartner) => {
                prerequisites.insert(ResourceType::Complex(ComplexResourceType::Robot), 1);
                prerequisites.insert(ResourceType::Complex(ComplexResourceType::Diamond), 1);
            }
        }

        Self {
            prerequisites,
            action: match what {
                ResourceType::Basic(a) => {Actions::Grab(a)},
                ResourceType::Complex(a) => {Actions::Craft(a)}
            },
        }
    }

    fn get_score(&self, memory: &mut Memory) -> Expectedscore {
        let item_score = self.action.get_expected_score();
        let move_path = memory.path_sanity(&memory.get_current_id(), &self.action.get_resource());
        let move_cost =  match (move_path) {
            None => {Expectedscore::new(EXPLORATION_COST)},
            Some(p) => {Expectedscore::new(p.len() as i32 * MOVEMENT_COST)}
        };
        item_score + move_cost        
    }
}

pub struct Brain {
    current_score: Score,
    plans: VecDeque<Plan>,
}

impl Brain {
    pub fn new() -> Brain {
        Self {
            current_score: Score {s: 0},
            plans: VecDeque::new(),
        }
    }
    pub fn clear_plans(&mut self) {
        self.plans.clear();
    }

    pub fn reset(&mut self) {
        self.current_score = Score {s: 0};
        self.plans.clear();
    }

    fn add_score(&mut self, other: Score) {
        self.current_score += other;
    }


    //TODO verify plan result and add score accordingly
    //TODO generate new plans, one very time a plan is concluded (successfully or not).
    //TODO choose best plan, only among those whose necessary requirements to work are met
    //TODO execute plans
    //TODO instead of the move into grab/craft plan, do cost calculation every time best plan is chosen assuming motion
}