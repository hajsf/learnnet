use serde::Serialize;
use serde_json;
use std::collections::BTreeSet;
use rocket;
use rocket::{Config, State};
use lib::blockchain::*;
use lib::consensus::Consensus;
use lib::transaction::*;
use std::sync::{RwLock};
use url::{Url};

mod api;
mod converters;

pub struct BlockchainState {
    pub blockchain: RwLock<Blockchain>
}

impl BlockchainState {
    pub fn new_with(difficulty: u64) -> BlockchainState {
        BlockchainState {
            blockchain: RwLock::new(Blockchain::new_with(difficulty))
        }
    }
}

//Requests
#[derive(Debug, Deserialize)]
pub struct NodeList {
    nodes: Vec<String>
}

//Responses
#[derive(Serialize)]
struct MineResult {
    message: String,
    index: usize,
    transactions: BTreeSet<Transaction>,
    proof: u64,
    previous_hash: String
}

#[derive(Serialize)]
struct ChainResult<'a> {
    chain: &'a BTreeSet<Block>,
    length: usize
}

#[derive(Serialize)]
struct RegisterNodeResponse {
    message: String,
    total_nodes: usize
}

pub fn init(rocket_config: Config, blockchain_state: BlockchainState) {
    rocket::custom(rocket_config, false)
    //rocket::ignite()
        .manage(blockchain_state)
        .mount("/", routes![
    
            mine, 
            new_transaction,
            chain,
            register_node,
            consensus 
            
        ]).launch();
}

//todo: respone as JSON - https://github.com/SergioBenitez/Rocket/blob/v0.3.3/examples/json/src/main.rs
#[get("/mine", format = "application/json")]
pub fn mine(state: State<BlockchainState>) -> Result<String, u32> {
    blockchain_op(&state, |b| Ok(format!("yo")) )
}


#[post("/transaction/new", format = "application/json", data = "<transaction>")]
pub fn new_transaction(transaction: Transaction, state: State<BlockchainState>) -> Result<String, u32> {
    blockchain_op(&state, |b| {
        let index = b.new_transaction(transaction.clone());
        return Ok(format!("Transaction added at block {}", index));
    })
}

#[get("/chain", format = "application/json")]
pub fn chain(state: State<BlockchainState>) -> Result<String, u32> {
    blockchain_op(&state, |b| {

        let chain = b.chain();
        let response = ChainResult {
            chain: chain,
            length: chain.len()
        };

        serialize(&response)
    })
}

#[post("/nodes/register", format = "application/json", data="<node_list>")]
pub fn register_node(node_list: NodeList, state: State<BlockchainState>) -> Result<String, u32> {
    return blockchain_op(&state, |b| {

        let mut node_urls = Vec::<Url>::with_capacity(node_list.nodes.len());

        //Validate - all or nothing
        for node in &node_list.nodes {
           match Url::parse(node) {
               Ok(parse_result) => node_urls.push(parse_result),
               Err(e) => {
                warn!("Failed to parse {} {:?}", node, e);
                return Err(400, /* all nodes must be valid */)
               }
           }
        }

        //Add
        for node_url in node_urls {
            b.register_node(node_url);
        }      

        let response = RegisterNodeResponse {
            message: String::from("New nodes have been added"),
            total_nodes: b.nodes().len(),
        };

        serialize(&response)     
    })
}

#[get("/nodes/resolve")]
pub fn consensus(state: State<BlockchainState>) -> Result<String, u32> {
    return blockchain_op(&state, |b| {
        let replaced = Consensus::resolve_conflicts(b);
        if replaced {
            return Ok(json!({
                "message": "Our chain was replaced",
                "new_chain": b.chain()
            }).to_string());
        }
        else
        {
            return Ok(json!({
                "message": "Our chain is authoritative",
                "chain": b.chain()
            }).to_string());
        }
    });
}

fn serialize<T>(response: &T) -> Result<String, u32> where T: Serialize {
    match serde_json::to_string(&response) {
        Ok(serialized) => Ok(serialized),
        Err(e) => {
            error!("serialize error: {:?}", e);
            return Err(500); //include reason?
        }
    }
}

///
/// Retrieves the blockchain from state, unlocks and executes the closure
/// 
fn blockchain_op<F>(state: &State<BlockchainState>, blockchain_op: F) -> Result<String, u32> 
    where F: Fn(&mut Blockchain) -> Result<String, u32> {
    
    let guard = state.blockchain.write();
    if guard.is_ok() {        
        let mut blockchain = guard.unwrap();
        let result = blockchain_op(&mut blockchain);
        return result;
    }
    error!("Couldn't acquire lock");
    Err(500)
}

#[cfg(test)]
mod tests {
    //These are only to support the state crate in testing. Could factor out
    use web::{BlockchainState};
    use lib::blockchain::Blockchain;

    #[test]
    fn mine() {
        let mut blockchain = Blockchain::new_with(1);
        // let result = ::web::mine_impl(&mut blockchain);
        // assert!(result.is_ok(), format!("Failed to mine {:?}", result));
        // println!("mine response: {}", result.unwrap());
    }
}