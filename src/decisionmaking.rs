use std::cmp::{Ordering, PartialOrd};
use std::collections::{HashMap, VecDeque};
use std::ops::{Add, AddAssign, SubAssign};
use std::cell::RefCell;
use std::rc::Rc;

use common_game::components::resource::{BasicResourceType,ComplexResourceType, GenericResource, ResourceType};

use crate::communication::PlanetComms;
use crate::InterruptOrder;
use crate::items::Inventory;
use crate::movement::{AnyResource, Memory, Thrusters};

const MOVEMENT_COST: i32 = -1;
pub const EXPECTED_RANDOM_MOVE: usize = 15;
const EXPLORATION_COST: i32 = EXPECTED_RANDOM_MOVE as i32 * MOVEMENT_COST;
const ITEM_MULT: i32 = 50;
const BASIC_VALUE: i32 = 1 * ITEM_MULT;
const COMPLEXITY_OFFSET: i32 = 1 * ITEM_MULT;
const WATER_VALUE: i32 = BASIC_VALUE * 2 + COMPLEXITY_OFFSET;
const DIAMOND_VALUE: i32 = BASIC_VALUE * 2 + COMPLEXITY_OFFSET;
const LIFE_VALUE: i32 = WATER_VALUE + BASIC_VALUE + COMPLEXITY_OFFSET;
const DOLPHIN_VALUE: i32 = LIFE_VALUE + BASIC_VALUE + COMPLEXITY_OFFSET;
const ROBOT_VALUE: i32 = LIFE_VALUE + BASIC_VALUE + COMPLEXITY_OFFSET;
const AIPARTNER_VALUE: i32 = ROBOT_VALUE + DIAMOND_VALUE + COMPLEXITY_OFFSET;

const MIN_PLANS_BEFORE_REPOP: usize = 3;
struct Score {s: i32}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        self.s += rhs.s;
    }
}

impl SubAssign for Score {
    fn sub_assign(&mut self, rhs: Self) {
        self.s -= rhs.s;
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
    fn positive(&self) -> bool {
        self.s > 0
    }
}

impl Add for Expectedscore {
    type Output = Expectedscore;

    fn add(self, rhs: Self) -> Self::Output {
        Expectedscore{s: self.s + rhs.s}
    }
}

impl PartialEq<Self> for Expectedscore {
    fn eq(&self, other: &Self) -> bool {
        self.s == other.s
    }
}

impl PartialOrd for Expectedscore {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.s.partial_cmp(&other.s)
    }
}

impl Clone for Expectedscore {
    fn clone(&self) -> Self {
        Expectedscore { s: self.s }
    }
}

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
            Actions::Grab(resource) => Actions::Grab(*resource),
            Actions::Craft(resource) => Actions::Craft(*resource),
        }
    }
}

struct Plan {
    prerequisites: HashMap<ResourceType, usize>,
    action: Actions,
}

impl Plan {
    fn new(what: ResourceType) -> Plan {
        log::trace!("LazyBoone: Devising new devious plan, for resource {:?}", what);

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

    fn get_score(&self, memory: &mut Memory, inventory: &Inventory) -> Expectedscore {
        log::trace!("LazyBoone: Doing some math to see if a plan is worth its while");

        let mut requirements_met = true;
        let keys = self.prerequisites.keys();
        for i in keys {
            requirements_met = (*self.prerequisites.get(i).expect("A key iterator somehow is not a key") <= inventory.has_resource(*i)) && requirements_met; //Only give points if the prerequisites are met
        }

        if requirements_met {
            let item_score = self.action.get_expected_score();
            let move_path = memory.path_sanity(&memory.get_current_id(), &self.action.get_resource());
            let move_cost = match move_path {
                None => { Expectedscore::new(EXPLORATION_COST) },
                Some(p) => { Expectedscore::new(p.len() as i32 * MOVEMENT_COST) }
            };
            item_score + move_cost
        } else {
            Expectedscore::new(0)
        }
    }
}

pub struct Brain {
    current_score: Score,
    last_failure: Option<AnyResource>,
    plans: VecDeque<Plan>,
    planet_comms: Rc<RefCell<PlanetComms>>
}

impl Brain {
    pub fn new(planet_comms: Rc<RefCell<PlanetComms>>) -> Brain {
        Self {
            current_score: Score {s: 0},
            last_failure: None,
            plans: VecDeque::new(),
            planet_comms
        }
    }

    pub fn reset(&mut self) {
        log::trace!("LazyBoone: Thinking it's time for a new start");
        self.current_score = Score {s: 0};
        self.plans.clear();
        self.last_failure = None;
    }

    fn best_plan(&self, memory: &mut Memory, inventory: &Inventory) -> Option<usize> {
        log::trace!("LazyBoone: Making the bast of what he came up with");

        let mut best_score = None;
        let mut best_scorer = None;
        for i in 0..self.plans.len() {
            if self.last_failure.is_none() || ( self.plans.len() == 1 || self.last_failure.clone().expect("Just checked it is not none") != self.plans.get(i).expect("i in between 0 an len so you'll never read this!").action.get_resource()) { //clone not to consume it, if the plan is the last failure it does not qualify, unless it is the only plan
                let score = self.plans[i].get_score(memory, inventory);
                if score.positive() && (best_score.is_none() || score > best_score.clone().expect("Best score is not None, but also not Some, law of excluded middle be damned")) { //The or short circuiting protects the unwrap
                    best_scorer = Some(i);
                    best_score = Some(score);
                }
            }
        }
        best_scorer
    }

    pub fn solve_best_plan(&mut self, thrusters: &mut Thrusters, inventory: &mut Inventory) -> Result<(), InterruptOrder> {
        log::trace!("LazyBoone: Walking the walk, doing the plan");

        let borrow_mem = &mut  thrusters.memory;
        let which = self.best_plan(borrow_mem, inventory);
        match which {
            Some(indx) => {
                let plan = self.plans.get(indx).expect("Somehow, the best plan is outside of the plan array");
                let resource = plan.action.get_resource();
                let res = thrusters.move_to(resource.clone());
                match res {
                    Ok(cost) => {
                        self.current_score -= Score{s: cost};
                        //We now try to get the item
                        match resource.into() {
                            ResourceType::Basic(gennable) => {
                                if let Some(res) = self.planet_comms.borrow().try_get(gennable) {
                                    inventory.put_in_bag(GenericResource::BasicResources(res));
                                    self.current_score += plan.action.get_expected_score().to_score();
                                    self.plans.remove(indx);
                                    self.last_failure = None;
                                    log::trace!("LazyBoone: Reaping what was sown");
                                } else {
                                    //Item not got, plan is not deleted
                                    self.last_failure = Some(self.plans.get(indx).expect("Really, after all this checking?").action.get_resource());
                                    log::trace!("LazyBoone: Feeling defeated but not dejected");
                                }

                            },
                            ResourceType::Complex(craftable) => {
                                if let Some(res) = self.planet_comms.borrow().try_craft(inventory, craftable) {
                                    inventory.put_in_bag(GenericResource::ComplexResources(res));
                                    self.current_score += plan.action.get_expected_score().to_score();
                                    self.plans.remove(indx);
                                    self.last_failure = None;
                                    log::trace!("LazyBoone: Enjoying the spoils of victory");
                                } else {
                                    //Item not got, plan is not deleted
                                    self.last_failure = Some(self.plans.get(indx).expect("Running out of witty things to say in these").action.get_resource());
                                    log::trace!("LazyBoone: Renewing hunger for success");
                                }
                            }
                        }
                        Ok(())
                    },
                    Err((ord, cost)) => {
                        log::trace!("LazyBoone: Unexpected order passed by the brain");
                        self.current_score -= Score{s: cost};
                        self.last_failure = None;
                        Err(ord)
                        //Plan not removed from Brain, no further action necessary
                    }
                }
            }
            None => {
                //Nothing to do, better poll just in case
                self.last_failure = None;
                log::trace!("LazyBoone: Idling, loitering even");
                Ok(())
            }
        }
    }

    fn plan_count(&self) -> usize {
        self.plans.len()
    }

    fn make_plan (&mut self, what: ResourceType) {
        let plan = Plan::new(what);
        log::trace!("LazyBoone: Making sure bases will be covered, eventually");
        for (i, n) in plan.prerequisites.iter() { //For every prerequisite
            for _ in  0..*n { //Repeat requisite amount of times
                self.make_plan(*i); //Recursively make the prerequisite plan
            }
        }
        self.plans.push_back(plan); //Commit the created plans
    }

    pub fn populate_plans (&mut self) {
        log::trace!("LazyBoone: Judging if some scheming is in order");
        if self.plan_count() <= MIN_PLANS_BEFORE_REPOP {
            log::trace!("LazyBoone: It is");
            self.make_plan(ResourceType::Complex(ComplexResourceType::AIPartner));
        }
    }
}