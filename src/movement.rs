pub const EXPECTED_RANDOM_COST: usize = 15;

use common_game::{utils::ID};
use common_game::components::resource::{ResourceType};

use std::{cell::RefCell, rc::Rc, hash::*};
use std::collections::{HashMap, HashSet, VecDeque};
use common_game::components::planet::PlanetType::{C, D};
use crossbeam_channel::unbounded;
use crate::movement::PlanetStatus::Explored;

enum PlanetStatus{
    Unexplored,
    Explored(PlanetContent)
}

impl PlanetStatus{
    fn get_content(&self) -> Option<&HashSet<ResourceType>> {
        match self {
            PlanetStatus::Unexplored => None,
            PlanetStatus::Explored(cont) => Some(cont.get_products())
        }
    }
}
struct Planet {
    id: ID,
    cont: PlanetStatus,
    adj: Vec<ID>
}
pub type Path = Vec<ID>;

struct PlanetContent {
    // TODO put all useful and knowable planet information here
    produces: HashSet<ResourceType>,
}

impl PlanetContent {
    fn new(produces: HashSet<ResourceType>) -> PlanetContent {
        PlanetContent{
            produces,
        }
    }

    fn get_products(&self) -> &HashSet<ResourceType> {
        &self.produces
    }
}

impl Planet {

    fn new(received_id: ID) -> Self {
        Planet {
            id: received_id,
            cont: PlanetStatus::Unexplored,
            adj: Vec::new()
        }
    }

    fn explored(&mut self, cont: PlanetContent) {
        self.cont = PlanetStatus::Explored(cont);
    }

    fn destroy(self) -> Vec<ID> {
        self.adj
    }
}

pub struct Memory {
    weights: HashMap<ID, HashMap<ResourceType, i32>>,
    map: HashMap<ID, Planet>,
}

impl Memory {
    fn new() -> Self {
        Self { map: HashMap::new(), weights: HashMap::new() }
    }

    fn insert_planet(&mut self, id: ID) {
        self.map.insert(id, Planet::new(id));
        self.weights.insert(id, HashMap::new());
    }

    fn forget_planet(&mut self, id: ID) {
        //TODO use width-first queueing (no repeats) on on planets in way defined below, then update ex-weight in order of addition to the queue
        //Planets with a weight greater than the neighbour that called them, recursively
        let mut update_queue = VecDeque::new(); //Non-repeating queue of all planets that have had any of their weights deleted, for updating
        let mut touched_update = HashSet::new();
        let mut weights_to_remove:Vec<(ID, ResourceType)> = Vec::new();

        touched_update.insert(id); //Prevents later from updating the dead planet again

        //Weight dependencies propagate upwards, so they must be uprooted
        for i in self.weights.get(&id).unwrap().keys() { //Repeat for all elements the planet had weights for
            let mut handle_queue = VecDeque::new(); //Non-repeating queue of all planets that have a weight dependent on the deleted planet
            let mut touched = HashSet::new();

            handle_queue.push_back(id);
            touched.insert(id);

            while !handle_queue.is_empty() {
                let current = handle_queue.pop_front().unwrap();
                if !touched.contains(&current) {
                    if !touched_update.contains(&current) {
                        touched_update.insert(current);
                        update_queue.push_back(current);
                    }

                    weights_to_remove.push((current, *i));

                    if let Some(current_weight) = self.weights.get(&current).unwrap().get(i) {
                        for ii in self.map.get(&current).unwrap().adj.iter() {
                            if !touched.contains(ii) {
                                if let Some(other_weight) = self.weights.get(ii).unwrap().get(i) {
                                    if current_weight > other_weight {
                                        handle_queue.push_back(*ii);
                                        touched.insert(*ii);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for (idr, whr) in weights_to_remove {
            self.weights.get_mut(&idr).unwrap().remove(&whr);
        }

        self.weights.remove(&id);

        //Remove forgotten planet from its adjacencies
        if let Some(temp) = self.map.remove(&id) {
            for i in temp.adj {
                for ii in 0..self.map.get(&i).unwrap().adj.len() {
                    if self.map.get(&i).unwrap().adj[ii] == id {
                        self.map.get_mut(&i).unwrap().adj.remove(ii);
                    }
                }
            }
        }

        for i in update_queue { //Updates all the other planets, now that the adjacency is removed
            self.update_planet(&i);
        }

        //From here on all weights should be guaranteed to be valid within Memory (but not necessarily really! you find out when you try and move there)
    }

    fn explore(&mut self, id: &ID) {
        //TODO include comms to get the resources of the planet, and adjacencies

        let mut newAdjs: Vec<ID> = Vec::new();

        for i in newAdjs.iter() {
            if !self.map.contains_key(i) {
                self.map.insert(*i, Planet::new(*i));
            }
        }
        self.map.get_mut(id).unwrap().adj.append(&mut newAdjs);

        let mut prods: HashSet<ResourceType> = HashSet::new();
        self.map.get_mut(id).unwrap().explored(PlanetContent::new(prods.clone()));

        for i in prods {
            self.update_resource(id, i, 0); //All resources it produces have distance zero
        }

        self.update_planet(id);
    }

    fn get_dist(&mut self, current_planet: &ID, resource: &ResourceType) -> Option<i32> {
        self.update_planet(current_planet);
        if let Some(dist) = self.weights.get(current_planet).unwrap().get(resource) {
            Some(dist.clone())
        } else {
            None
        }
    }

    fn update_planet(&mut self, id: &ID) {
        fn update_weights(W1: &HashMap<ResourceType, i32>, W2: &mut HashMap<ResourceType, i32>) {
            for i in W1.keys() {
                match (W1.get(i), W2.get(i)) {
                    (None, _) => { /*Nothing to do here, should be unreachable*/ },
                    (Some(w), None) => { W2.insert(*i, w + 1); },
                    (Some(w1), Some(w2)) => {
                        if w1 + 1 < *w2 {
                            W2.insert(*i, w1 + 1);
                        }
                    },
                }
            }
        }

        let mut newWeights: HashMap<ResourceType, i32> = HashMap::new();
        for i in self.map.get(id).unwrap().adj.iter() {
            match self.weights.get(i) {
                None => { /*Nothing to do*/ },
                Some(W) => { update_weights(W, &mut newWeights) },
            }
        }

        //Set weights of locally-produced resources to zero
        for i in self.map.get(id).unwrap().cont.get_content().unwrap().iter() {
            newWeights.insert(*i, 0);
        }

        //Override old weights
        self.weights.insert(id.clone(), newWeights);
    }

    fn update_resource(&mut self, id: &ID, resource: ResourceType, dist: i32) {
        self.weights.get_mut(id).unwrap().insert(resource, dist);
    }


    fn next_step (&mut self, start: &ID, what: &ResourceType) -> Option<ID> { //Wish I could've made this a closure inside path_sanity, but rust doesn't like the self borrows
        let mut foundID = None;
        match self.get_dist(start, what) { //Updates the planet
            Some(dist) => {
                let mut candidates = Vec::new();
                for i in self.map.get(start).unwrap().adj.iter() {
                    candidates.push(*i);
                }
                while (candidates.len() > 0) {
                    let candidate = candidates.pop().unwrap();
                    if self.get_dist(&candidate, what) == Some(dist - 1) {
                        foundID = Some(candidate);
                    }
                }
                foundID //Someone was found, yay! or the planet was destroyed, bad
            }
            None => { None } //You are on a planet adjacent to the destroyed one, so no need to search around
        }
    }


    fn path_sanity(&mut self, start: &ID, what: &ResourceType) -> Option<Path> {
        enum PathResult {
            None,
            Arrived,
            New(Path),
            Beaten
        }

        let first_check = match self.get_dist(start, what) {
            Some(dist) => { //Existing weights are guaranteed by forget_planet
                if dist == 0 {
                    PathResult::Arrived
                } else {
                    PathResult::Beaten
                }
            },
            None => {
                let mut i = 1;
                let mut explored: Vec<HashMap<ID, ID>> = Vec::new(); //ID of the planet, ID of the precursor
                let mut touched: HashSet<ID> = HashSet::new();
                touched.insert(start.clone());

                let mut found_flag = false;


                let mut first_gen: HashMap<ID, ID> = HashMap::new();
                first_gen.insert(start.clone(), start.clone());

                explored.push(first_gen);

                while (i <= EXPECTED_RANDOM_COST && !found_flag) {
                    let mut new_gen: HashMap<ID, ID> = HashMap::new();
                    for ii in explored[i].keys() {
                        for iii in self.map.get(ii).unwrap().adj.iter() {
                            if !touched.contains(&iii) {
                                new_gen.insert(iii.clone(), ii.clone());
                                touched.insert(iii.clone());
                            }
                        }
                    }
                    explored.push(new_gen);
                    i += 1;
                    for ii in explored[i].keys() {
                        if self.weights.get(ii).unwrap().contains_key(what) {
                            found_flag = true;
                        }
                    }
                }
                if found_flag {
                    let mut path: Path = Path::new();

                    let mut lowest_dist: Option<ID> = None;
                    for ii in explored[i].keys() {
                        if self.weights.get(ii).unwrap().contains_key(what) {
                            if lowest_dist.is_none() || self.weights.get(ii).unwrap().get(what).unwrap() < self.weights.get(&lowest_dist.unwrap()).unwrap().get(what).unwrap() {
                                lowest_dist = Some(*ii);
                            }
                        }
                    }

                    if lowest_dist.is_none() {
                        panic!("How?! lowest_dist is none")
                    }

                    let range = 1..i; //Precalculate range so we can change i

                    //Now we know where we want to go
                    for _ in range {
                        path.insert(0, lowest_dist.unwrap()); //Insert in the leftmost position
                        lowest_dist = Some(*explored[i].get(&lowest_dist.unwrap()).unwrap()); //Take the precursor of lowest dist
                        i -= 1; //Lower i to look at the previous generation, then repeat
                    }
                    //i should be 0 here
                    path.remove(0); //We remove the first element, that should be "start"
                    PathResult::New(path)
                } else {
                    //No planet in the discovered galaxy matches requirement
                    PathResult::None
                }
            }
        };

        let mut out_path = Path::new();

        match first_check {
            PathResult::Arrived => {Some(out_path)}
            PathResult::Beaten => {
                //Next_step is guaranteed because the forget planet function forces all planets to update correctly
                let mut current = self.next_step(start, what);
                while current != None {
                    out_path.push(current.unwrap());
                    current =self.next_step(&current.unwrap(), what);
                }
                Some(out_path)
            }
            PathResult::New(temp_path) => {
                //Weights are assigned to "normalize" the path into a beaten one, then the function is called again as-is to compute the full path
                let mut offset = temp_path.len()-1; //Given N elements, should be a range N-1..0

                for i in temp_path {
                    self.update_resource(&i, what.clone(), offset as i32);
                    offset -= 1;
                }

                //Calls again to go into the PathResult::Beaten branch, and then propagates the return
                if let Some(some_path) =self.path_sanity(&start, &what) {
                    Some(some_path)
                } else {None}
            }
            PathResult::None => {
                //Shoot out and explore, we don't have a planet for this explored
                //Go to the first adjacent unexplored planet (explore it), or not having one to a random explored adjacent planet and repeat
                //Continue going randomly (unxeplored first) and stop when you find a suitable planet
                //Ideally, store the planets walked in a LIFO queue and run an update on them when a planet is found
                None
            }
        }
    }
}