use crate::atomicfile;
use display_as::{display, with_template, DisplayAs, HTML, UTF8};
use futures::{FutureExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use warp::reply::Reply;
use warp::{path, Filter};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Character {
    code: String,
    name: String,
    spell_points: Option<(i32, i32)>,
    psi: Option<(i32, i32)>,
    sections: Vec<Section>,
}
#[with_template("[%" "%]" "character.html")]
impl DisplayAs<HTML> for Character {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Section {
    title: String,
    id: String,
    content: String,
    table: Vec<Row>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Row {
    id: String,
    action: Action,
    items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Item {
    id: String,
    value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum Action {
    Spell { name: String, points: i32 },
    Psychic { name: String, psi: i32 },
    Generic { name: String },
}

impl Character {
    fn slug(&self) -> String {
        self.name
            .replace("'", "-")
            .replace(" ", "-")
            .replace("\"", "-")
            .replace("\\", "-")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Party {
    code: String,
    characters: Vec<Character>,
}
#[with_template("[%" "%]" "party.html")]
impl DisplayAs<HTML> for Party {}

impl Party {
    fn read(code: &str) -> Self {
        if let Ok(f) = ::std::fs::File::open(format!("data/{}", code)) {
            if let Ok(mut s) = serde_yaml::from_reader::<_, Vec<Character>>(&f) {
                // Anything that has an empty name should just be deleted...
                s.retain(|x| x.name.len() > 0);
                return Party {
                    code: code.to_string(),
                    characters: s,
                };
            }
        }
        Party {
            code: code.to_string(),
            characters: Vec::new(),
        }
    }
    fn save(&self) {
        let f = atomicfile::AtomicFile::create(format!("data/{}", self.code))
            .expect("error creating save file");
        serde_yaml::to_writer(&f, &self.characters).expect("error writing yaml")
    }
    fn lookup(&self, name: &str) -> Character {
        for c in self.characters.iter() {
            if c.name == name {
                return c.clone();
            }
        }
        return Character {
            code: self.code.clone(),
            name: name.to_string(),
            spell_points: None,
            psi: None,
            sections: Vec::new(),
        };
    }
}

type Editors = std::sync::Arc<
    RwLock<
        std::collections::HashMap<
            String,
            Vec<mpsc::UnboundedSender<Result<warp::ws::Message, warp::Error>>>,
        >,
    >,
>;

pub fn sheets() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let editors = Editors::default();
    // Turns our "state" into a new filter.
    let editors = warp::any().map(move || editors.clone());

    let index = path!("sheets").map(|| {
        println!("I am doing index.");
        display(HTML, &Index).into_response()
    });
    let character =
        path!("sheets" / String / String).map(|code: String, character_name: String| {
            let character_name = percent_encoding::percent_decode(character_name.as_bytes())
                .decode_utf8()
                .unwrap();
            let code = percent_encoding::percent_decode(code.as_bytes())
                .decode_utf8()
                .unwrap();
            let party = Party::read(&code);
            display(HTML, &party.lookup(&character_name)).into_response()
        });
    let party = path!("sheets" / String).map(|code: String| {
        println!("Party: {}", code);
        let code = percent_encoding::percent_decode(code.as_bytes())
            .decode_utf8()
            .unwrap();
        let party = Party::read(&code);
        display(HTML, &party).into_response()
    });
    let sock = path!("sheets" / "ws" / String)
        .and(warp::ws())
        .and(editors)
        .map(|code: String, ws: warp::ws::Ws, editors| {
            ws.on_upgrade(move |socket| editor_connected(code, socket, editors))
        });
    sock.or(character).or(party).or(index)
}

async fn editor_connected(code: String, ws: warp::ws::WebSocket, editors: Editors) {
    println!("Someone connected.");
    // Split the socket into a sender and receive of messages.
    let (user_ws_tx, mut user_ws_rx) = ws.split();

    // Use an unbounded channel to handle buffering and flushing of messages
    // to the websocket...
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::task::spawn(rx.forward(user_ws_tx).map(|result| {
        if let Err(e) = result {
            eprintln!("websocket send error: {}", e);
        }
    }));

    {
        // Save the sender in our list of connected users.
        let mut e = editors.write().await;
        if e.get(&code).is_none() {
            e.insert(code.clone(), Vec::new());
        }
        e.get_mut(&code).unwrap().push(tx);
        e.get(&code).unwrap().len();
    }

    // Return a `Future` that is basically a state machine managing
    // this specific user's connection.

    // Make an extra clone to give to our disconnection handler...
    let editors2 = editors.clone();

    // Every time the user sends a message, broadcast it to
    // all other users...
    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error: {}", e);
                break;
            }
        };
        process_message(&code, msg, &editors).await;
    }

    // user_ws_rx stream will keep processing as long as the user stays
    // connected. Once they disconnect, then...
    ws_disconnected(&editors2).await;
}

async fn process_message(code: &str, msg: warp::ws::Message, editors: &Editors) {
    let mut to_remove = Vec::new();
    for tx in editors.read().await.get(code).iter().flat_map(|x| x.iter()) {
        println!("Sending {:?} to {:?}", msg, tx);
        if let Err(_disconnected) = tx.send(Ok(msg.clone())) {
            // The tx is disconnected, our `user_disconnected` code
            // should be happening in another task, nothing more to
            // do here.
            println!("Websocket disconnected?");
            to_remove.push(tx.clone());
        }
    }
}

async fn ws_disconnected(_editors: &Editors) {}

struct Index;
#[with_template("[%" "%]" "index.html")]
impl DisplayAs<HTML> for Index {}
