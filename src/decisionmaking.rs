use std::f32::consts::E;
use std::ops::{Add, AddAssign};
use common_game::components::resource::BasicResourceType as BasicResourceIdea;
use common_game::components::resource::ComplexResourceType as ComplexResourceIdea;

use crate::movement::Path;
use crate::movement::EXPECTED_RANDOM_COST;
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
    Move(Path), //Move along a path, thrown out in case of error (forget the missing planet and re-do the pathfinding)
    Explore,
    Grab(BasicResourceIdea), //insert item type here, should create it's move command on creation, using the weights
    Craft(ComplexResourceIdea), //insert item type here, like grab should create move
}

impl Actions {
    fn get_expected_score(&self) -> Expectedscore {
        match self {
            Actions::Move(Path) => {Expectedscore::new(Path.len() as i32 * -1)}
            Actions::Explore => {Expectedscore::new(-1 * EXPECTED_RANDOM_COST as i32)}
            _=>{todo!()}
        }
    }

    fn confirm_result(self, result: ActionResult, current_score: &mut Score) {
        match result {
            Ok(_) => {*current_score += self.get_expected_score().to_score()},
            Err(_) => {}
        }
    }
}

impl Clone for Actions {
    fn clone(&self) -> Self {
        match self {
            Actions::Move(Path) => Actions::Move(Path.clone()),
            Actions::Explore => Actions::Explore,
            Actions::Grab(resource) => Actions::Grab(resource.clone()),
            Actions::Craft(resource) => Actions::Craft(resource.clone()),
        }
    }
}

struct Plan {
    actions: Vec<Actions>,
    gain: Expectedscore
}

impl Plan {
    fn new(vec: Vec<Actions>) -> Plan {
        let acc:Expectedscore = vec.clone().into_iter().fold(Expectedscore::new(0), |acc: Expectedscore, s| acc + s.get_expected_score());
        Plan{actions: vec, gain: acc}
    }
}
