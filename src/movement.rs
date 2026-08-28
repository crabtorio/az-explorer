use common_game::{utils::ID};
use common_game::components::resource::{BasicResourceType, ComplexResourceType, ResourceType};

use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::cell::RefCell;

use crate::communication::{OrchestratorComms, PlanetComms};
use crate::decisionmaking::EXPECTED_RANDOM_MOVE;
use crate::InterruptOrder;

enum PlanetStatus{
    Unexplored,
    Explored(PlanetContent)
}

impl PlanetStatus{
    fn get_content(&self) -> Option<&HashSet<AnyResource>> {
        match self {
            PlanetStatus::Unexplored => None,
            PlanetStatus::Explored(cont) => Some(cont.get_products())
        }
    }
}

impl PartialEq for PlanetStatus{
    fn eq(&self, other: &PlanetStatus) -> bool { //Shallow matching since it is just to distinguish between explored and not
        matches!((self, other),
            (PlanetStatus::Unexplored, PlanetStatus::Unexplored) |
            (PlanetStatus::Explored(_), PlanetStatus::Explored(_))
        )
    }
}
struct Planet {
    id: ID,
    cont: PlanetStatus,
    adj: Vec<ID>
}
pub type Path = Vec<ID>;

struct PlanetContent {
    produces: HashSet<AnyResource>,
}

impl PlanetContent {
    fn new(produces: HashSet<AnyResource>) -> PlanetContent {
        PlanetContent{
            produces,
        }
    }

    fn get_products(&self) -> &HashSet<AnyResource> {
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

}


#[derive(Clone, PartialEq, Eq, Hash)]
pub enum AnyResource {
    Oxygen,
    Hydrogen,
    Carbon,
    Silicon,
    Water,
    Life,
    Dolphin,
    Robot,
    Diamond,
    AiPartner
}

impl From<&BasicResourceType> for AnyResource {
    fn from(resource_type: &BasicResourceType) -> Self {
        match resource_type {
            BasicResourceType::Oxygen => AnyResource::Oxygen,
            BasicResourceType::Hydrogen => AnyResource::Hydrogen,
            BasicResourceType::Carbon => AnyResource::Carbon,
            BasicResourceType::Silicon => AnyResource::Silicon
        }
    }
}

impl From<&ComplexResourceType> for AnyResource {
    fn from(resource_type: &ComplexResourceType) -> Self {
        match resource_type {
            ComplexResourceType::Water => AnyResource::Water,
            ComplexResourceType::Life => AnyResource::Life,
            ComplexResourceType::Dolphin => AnyResource::Dolphin,
            ComplexResourceType::Robot => AnyResource::Robot,
            ComplexResourceType::Diamond => AnyResource::Diamond,
            ComplexResourceType::AIPartner => AnyResource::AiPartner
        }
    }
}

impl Into<ResourceType> for AnyResource {
    fn into(self) -> ResourceType {
        match self {
            AnyResource::Oxygen => ResourceType::Basic(BasicResourceType::Oxygen),
            AnyResource::Hydrogen=> ResourceType::Basic(BasicResourceType::Hydrogen),
            AnyResource::Carbon=> ResourceType::Basic(BasicResourceType::Carbon),
            AnyResource::Silicon=> ResourceType::Basic(BasicResourceType::Silicon),
            AnyResource::Water=> ResourceType::Complex(ComplexResourceType::Water),
            AnyResource::Life=> ResourceType::Complex(ComplexResourceType::Life),
            AnyResource::Dolphin=> ResourceType::Complex(ComplexResourceType::Dolphin),
            AnyResource::Robot=> ResourceType::Complex(ComplexResourceType::Robot),
            AnyResource::Diamond=> ResourceType::Complex(ComplexResourceType::Diamond),
            AnyResource::AiPartner=> ResourceType::Complex(ComplexResourceType::AIPartner)
        }
    }
}

pub struct Memory {
    current: Rc<RefCell<ID>>,
    weights: HashMap<ID, HashMap<AnyResource, i32>>,
    map: HashMap<ID, Planet>,
    planet_comms: Rc<RefCell<PlanetComms>>,
    orchestrator_comms: Rc<RefCell<OrchestratorComms>>,
}

impl Memory {
    pub fn new(current: Rc<RefCell<ID>>, orchestrator_comms: Rc<RefCell<OrchestratorComms>>, planet_comms: Rc<RefCell<PlanetComms>>) -> Self {
        let mut map: HashMap<ID, Planet> = HashMap::new();
        map.insert(*current.borrow(), Planet::new(*current.borrow()));
        Self { current, map, weights: HashMap::new(), planet_comms: planet_comms.clone(), orchestrator_comms: orchestrator_comms.clone()}
    }

    pub fn forget_all(&mut self) {
        log::trace!("LazyBoone: Banged his head too hard");
        self.weights.clear();
        self.map.clear();
    }
    pub fn get_current_id(&self) -> ID {
        *self.current.borrow()
    }
    pub(crate) fn override_current_id(&mut self, id: ID){
        *self.current.borrow_mut() = id;
    }
    fn insert_planet(&mut self, id: ID) {
        log::trace!("LazyBoone: Noticing a new planet, naming it {:?}", id);
        self.map.insert(id, Planet::new(id));
        self.weights.insert(id, HashMap::new());
    }

    fn forget_planet(&mut self, id: &ID) {
        log::trace!("LazyBoone: Realizing on second thought planet {:?} is a silly place", id);
        //Planets with a weight greater than the neighbour that called them, recursively
        let mut update_queue = VecDeque::new(); //Non-repeating queue of all planets that have had any of their weights deleted, for updating
        let mut touched_update:HashSet<ID> = HashSet::new();
        let mut weights_to_remove: Vec<(ID, AnyResource)> = Vec::new();

        touched_update.insert(*id); //Prevents later from updating the dead planet again
        if !self.map.contains_key(id) {return;}

        //Weight dependencies propagate upwards, so they must be uprooted. If the planet was unexploreed then there was no such thing
        if self.map.get(id).expect("ID-d is none only occasionally in forget function, neat").cont != PlanetStatus::Unexplored {
            for i in self.weights.get(id).expect("ID is none only occasionally in forget function, neat").keys() { //Repeat for all elements the planet had weights for
                let mut handle_queue: VecDeque<ID> = VecDeque::new(); //Non-repeating queue of all planets that have a weight dependent on the deleted planet
                let mut touched: HashSet<ID> = HashSet::new();

                handle_queue.push_back(*id);
                touched.insert(*id);

                while !handle_queue.is_empty() {
                    let current = handle_queue.pop_front().expect("Pop failed on necessarily nonempty queue");
                    if !touched.contains(&current) {
                        if !touched_update.contains(&current) {
                            touched_update.insert(current);
                            update_queue.push_back(current);
                        }

                        weights_to_remove.push((current, i.clone()));

                        if let Some(current_weight) = self.weights.get(&current).expect("If the planet is explored, the weights are supposed to be guaranteed").get(i) {
                            let temp = self.map.get(&current).expect("An inexistent planet should be guaranteed to have no weights, but got here anyway somehow").adj.clone();
                            for ii in temp {
                                if !touched.contains(&ii) &&
                                    let Some(other_weight) = self.weights.get(&ii).expect("If the planet is explored, the weights are supposed to be guaranteed").get(i) &&
                                    current_weight > other_weight {
                                        handle_queue.push_back(ii);
                                        touched.insert(ii);
                                }
                            }
                        }
                    }
                }
            }
        }
        for (idr, whr) in weights_to_remove {
                self.weights.get_mut(&idr).expect("Somehow, a planet with no weights had weights in need of removal").remove(&whr);
        }

        self.weights.remove(id);



        //Remove forgotten planet from its adjacencies
        if let Some(temp) = self.map.remove(id) {
            for i in temp.adj {
                for ii in 0..self.map.get(&i).expect("Every planet should be put in map as soon an an adjacent one is explored").adj.len() {
                    if self.map.get(&i).expect("Every planet should be put in a map as soon as an adjacent one is explored").adj[ii] == *id {
                        self.map.get_mut(&i).expect("Every planet should be put in the map as soon as an adjacent one is explored").adj.remove(ii);
                    }
                }
            }
        } else {
            panic!("How are you forgetting a planet that doesn't exist?");
        }

        for i in update_queue { //Updates all the other planets, now that the adjacency is removed
            self.update_planet(&i);
        }

        //From here on all weights should be guaranteed to be valid within Memory (but not necessarily really! you find out when you try and move there)
    }

    fn is_explored(&self, id: &ID) -> bool {
        self.map.get(id).expect("Planet doesn't exist?").cont != PlanetStatus::Unexplored
    }

    pub fn explore(&mut self) -> Result<(), InterruptOrder> {
        log::trace!("LazyBoone: Annotating interesting sights");
        let res = self.orchestrator_comms.borrow().get_adjs();
        let now = *self.current.borrow();
        if res.is_err() {
            return Err(res.expect_err("Res is Err but also not"));
        }
        let mut new_adjs = res.ok().expect("Res is not Err but also not Ok");

        for i in new_adjs.iter() {
            if !self.map.contains_key(i) {
                self.map.insert(*i, Planet::new(*i));
            }
        }

        self.map.get_mut(&now).expect("Current planet is somehow not adjacent to any planet explored").adj.append(&mut new_adjs);

        let mut prods: HashSet<AnyResource> = HashSet::new();

        for i in self.planet_comms.borrow().get_prods() {
            prods.insert(i);
        }
        self.map.get_mut(&now).expect("Planet somehow doesn't exist, even though it is guaranteed by memory creation and expansion").explored(PlanetContent::new(prods.clone()));

        self.weights.insert(now, HashMap::new());

        for i in prods {
            self.update_resource(&now, i, 0); //All resources it produces have distance zero
        }
        self.update_planet(&now);
        Ok(())
    }

    fn dist_from_here(&mut self, resource: &AnyResource) -> Result<Option<&i32>, InterruptOrder> {
        log::trace!("LazyBoone: Looking at the map");
        if !self.is_explored(&self.current.borrow()) {
            self.explore()?
        }
        Ok(self.weights.get(&self.current.borrow()).expect("Just explored to guarantee there would be one").get(resource))
    }

    fn get_dist(&mut self, current_planet: &ID, resource: &AnyResource) -> Option<i32> {
        log::trace!("LazyBoone: Using the fancy compass");
        if !self.is_explored(current_planet) {
            return None
        }
        self.update_planet(current_planet);
        self.weights.get(current_planet).expect("Planet will always be explored by now, and thus should have a weight").get(resource).copied()
    }

    fn update_planet(&mut self, id: &ID) {
        log::trace!("LazyBoone: Inking new info");
        fn update_weights(w1: &HashMap<AnyResource, i32>, w2: &mut HashMap<AnyResource, i32>) {
            for i in w1.keys() {
                match (w1.get(i), w2.get(i)) {
                    (None, _) => { /*Nothing to do here, should be unreachable*/ },
                    (Some(innie1), None) => { w2.insert(i.clone(), innie1 + 1); },
                    (Some(innie1), Some(innie2)) => {
                        if innie1 + 1 < *innie2 {
                            w2.insert(i.clone(), innie1 + 1);
                        }
                    },
                }
            }
        }

        let mut new_weights: HashMap<AnyResource, i32> = HashMap::new();
        for i in self.map.get(id).expect("Wrong ID in update_planet somehow").adj.iter() {
            match self.weights.get(i) {
                None => { /*Nothing to do*/ },
                Some(w) => { update_weights(w, &mut new_weights) },
            }
        }

        //Set weights of locally-produced resources to zero
        for i in self.map.get(id).expect("Somehow the planet doesn't exist, despite being updated").cont.get_content().expect("Somehow the planet doesn't have a content, despite being explored").iter() {
            new_weights.insert(i.clone(), 0);
        }

        //Override old weights
        self.weights.insert(*id, new_weights);
    }

    fn update_resource(&mut self, id: &ID, resource: AnyResource, dist: i32) {
        self.weights.get_mut(id).expect("Wrong planet ID to update a resource").insert(resource, dist);
    }


    fn next_step (&mut self, start: &ID, what: &AnyResource) -> Option<ID> { //Wish I could've made this a closure inside path_sanity, but rust doesn't like the self borrows
        log::trace!("LazyBoone: Finding the next planet to go");
        let mut found_id = None;
        match self.get_dist(start, what) { //Updates the planet
            Some(dist) => {
                let mut candidates = Vec::new();
                for i in self.map.get(start).expect("Wrong starting location ID for next_step").adj.iter() {
                    candidates.push(*i);
                }
                while let Some(candidate) = candidates.pop() {
                    let cand_res = self.get_dist(&candidate, what);
                    if cand_res == Some(dist-1) {
                        found_id = Some(candidate);
                    }
                }
                found_id //Someone was found, yay! or the planet was destroyed, bad
            }
            None => { None } //You are on a planet adjacent to the destroyed one, or an unexplored one, so no need to search around
        }
    }

    pub fn path_sanity(&mut self, start: &ID, what: &AnyResource) -> Option<Path> {
        log::trace!("LazyBoone: Making sure he's not about to get lost");
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
                let mut i = 0;
                let mut explored: Vec<HashMap<ID, ID>> = Vec::new(); //ID of the planet, ID of the precursor
                let mut touched: HashSet<ID> = HashSet::new();
                touched.insert(*start);

                let mut found_flag = false;


                let mut first_gen: HashMap<ID, ID> = HashMap::new();
                first_gen.insert(*start, *start);

                explored.push(first_gen);

                while i <= EXPECTED_RANDOM_MOVE && !found_flag {
                    let mut new_gen: HashMap<ID, ID> = HashMap::new();
                    for ii in explored[i].keys() {
                        for iii in self.map.get(ii).expect("Key iterator of array of explored planets is somehow not a valid key for planets").adj.iter() {
                            if !touched.contains(iii) {
                                new_gen.insert(*iii, *ii);
                                touched.insert(*iii);
                            }
                        }
                    }
                    explored.push(new_gen);
                    i += 1;
                    for ii in explored[i].keys() {
                        if self.is_explored(ii) && self.weights.get(ii).expect("Explored planets should always have a weight").contains_key(what) {
                            found_flag = true;
                        }
                    }
                }
                if found_flag {
                    let mut path: Path = Path::new();

                    let mut lowest_dist: Option<ID> = None;
                    for ii in explored[i].keys() {
                        if self.weights.get(ii).expect("Explored planets should always have a weight").contains_key(what) &&
                            lowest_dist.is_none() || self.weights.get(ii)
                            .expect("Explored planets should always have a weight").get(what)
                            .expect("Somehow the planet does not have a specific weight for the material in question")
                            <
                            self.weights.get(&lowest_dist.expect("The list of possible distances was somehow empty"))
                            .expect("A canditate exists but the ID was wrong somehow").get(what).expect("Candidate got past as the best distance without having one")
                        {
                                lowest_dist = Some(*ii);
                        }
                    }

                    if lowest_dist.is_none() {
                        panic!("How?! lowest_dist is none")
                    }

                    let range = 1..i; //Precalculate range so we can change i

                    //Now we know where we want to go
                    for _ in range {
                        path.insert(0, lowest_dist.expect("Candidate snuck by being None, meaning the possibilities list was empty")); //Insert in the leftmost position
                        lowest_dist = Some(*explored[i].get(&lowest_dist.expect("No Candidates")).expect("Candidate doesn't exist, somehow")); //Take the precursor of lowest dist
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
                while let Some(innie) = current {
                    out_path.push(innie);
                    current =self.next_step(&innie, what);
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
                self.path_sanity(start, what)
            }
            PathResult::None => {
                //Shoot out and explore, we don't have a planet for this explored
                //Go to the first adjacent unexplored planet (explore it), or not having one to a random explored adjacent planet and repeat
                //Continue going randomly (unexplored first) and stop when you find a suitable planet
                //Ideally, store the planets walked in a LIFO queue and run an update on them when a planet is found
                None
            }
        }
    }
}

pub struct Thrusters {
    pub memory:Memory,
    orchestrator_comms: Rc<RefCell<OrchestratorComms>>,
}

impl Thrusters {
    pub fn new(memory: Memory, orchestrator_comms: Rc<RefCell<OrchestratorComms>>) -> Self {
        Self {
            memory,
            orchestrator_comms: orchestrator_comms.clone()
        }
    }
    
    pub fn make_path(&mut self, what: AnyResource) -> Option<Path> {
        let now = *self.memory.current.borrow();
        self.memory.path_sanity(&now, &what)
    }
    
    pub fn move_to (&mut self, what: AnyResource) -> Result<i32, (InterruptOrder, i32)> {
        log::trace!("LazyBoone: Firing up the engines to move");
        let now = *self.memory.current.borrow();
        let ret = self.memory.path_sanity(&now, &what);

        if let Some(path) = ret {
            //Found a path
            let mut counter = 0;
            for i in path {
                let res = self.orchestrator_comms.borrow().request_move(i);
                if let Err(ord) = res {
                    //Move failed
                    return Err((ord, counter))
                } else {
                    counter += 1;
                }
            }
            Ok(counter)
        } else {
            self.explore(what)
        }
    }

    fn explore(&mut self, what: AnyResource) -> Result<i32, (InterruptOrder, i32)> {
        log::trace!("LazyBoone: Exploing uncharted land");
        //Explore until finding the requested resource, then return the amount of movements done
        let mut counter:i32 = 0;

        while counter <= (EXPECTED_RANDOM_MOVE * 2) as i32 &&
                match self.memory.dist_from_here(&what) {
                    Ok(None) => { true },
                    Ok(Some(_)) => { false },
                    Err(ord) => { return Err((ord, counter)) }
                }
        {
            counter += 1;
            let mut found = None;
            let adjs = self.memory.map.get(&self.memory.get_current_id()).expect("Current planet is somehow not in map").adj.clone();
            for i in adjs.iter() {
                if found.is_none() && let PlanetStatus::Unexplored = self.memory.map.get(i).expect("Plane adjacency was not generated properly, as an ID exists with no planet").cont {
                    found = Some(i);
                }
            }

            if let Some(found) = found {
                let res = self.orchestrator_comms.borrow().request_move(*found);
                if res.is_ok() {
                    //If new planet, explore, else move failed
                    if let Err(ord) = self.memory.explore() {
                        return Err((ord, counter))
                    }
                } else {
                    let ord = res.expect_err("res is Err but also not Err");

                    if ord == InterruptOrder::None {
                        self.memory.forget_planet(found);
                    }
                    return Err((ord, counter - 1))
                }
            } else {
                let next_step = (rand::random::<i32>() % adjs.len() as i32) as usize; //Pseudo-random to guarantee no infinite loops
                let now = *self.memory.current.borrow();
                let res = self.orchestrator_comms.borrow().request_move(self.memory.map.get(&now).expect("Current planet is somehow not mapped").adj[next_step]);
                if let Err(ord) = res {
                    //request_move failed
                    if ord == InterruptOrder::None {
                        let what = self.memory.map.get(&now).expect("Current planet is somehow not mapped").adj[next_step];
                        self.memory.forget_planet(&what);
                    }
                    return Err((ord, counter));
                }
            }
        }
        Ok(counter)
    }
}