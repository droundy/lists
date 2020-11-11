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
    title_id: String,
    content: String,
    content_id: String,
    table: Vec<Row>,
}
#[with_template("[%" "%]" "section.html")]
impl DisplayAs<HTML> for Section {}

impl Section {
    fn change(&mut self, change: &Change) -> bool {
        if change.kind == "change" {
            if change.id == self.title_id {
                self.title = change.html.clone();
                return true;
            } else if change.id == self.content_id {
                self.content = change.html.clone();
                return true;
            }
        }
        false
    }
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
    fn read(code: &str, name: &str) -> Self {
        if let Ok(f) = ::std::fs::File::open(format!("data/{}/{}.yaml", code, name)) {
            if let Ok(c) = serde_yaml::from_reader::<_, Character>(&f) {
                return c;
            }
        }
        Character {
            code: code.to_string(),
            name: name.to_string(),
            spell_points: None,
            psi: None,
            sections: Vec::new(),
        }
    }
    fn save(&self) {
        std::fs::create_dir_all(format!("data/{}", self.code)).unwrap();
        let f = atomicfile::AtomicFile::create(format!("data/{}/{}.yaml", self.code, self.name))
            .expect("error creating save file");
        serde_yaml::to_writer(&f, self).expect("error writing yaml")
    }
    fn change(&mut self, change: &Change) -> bool {
        for c in self.sections.iter_mut() {
            if c.change(&change) {
                return true;
            }
        }
        if change.kind == "change" {
            self.sections.push(Section {
                title: change.html.clone(),
                title_id: change.id.clone(),
                content: "Edit me".to_string(),
                content_id: memorable_wordlist::camel_case(44),
                table: Vec::new(),
            });
            println!("I just pushed a new section {:?}", change.html);
            return true;
        }
        false
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
            let mut c = Character::read(&code, &character_name);
            c.sections.push(Section {
                title: "Edit this to create a new section".to_string(),
                title_id: memorable_wordlist::camel_case(44),
                content: "".to_string(),
                content_id: memorable_wordlist::camel_case(44),
                table: Vec::new(),
            });
            display(HTML, &c).into_response()
        });
    // let party = path!("sheets" / String).map(|code: String| {
    //     println!("Party: {}", code);
    //     let code = percent_encoding::percent_decode(code.as_bytes())
    //         .decode_utf8()
    //         .unwrap();
    //     let party = Party::read(&code);
    //     display(HTML, &party).into_response()
    // });
    let sock = path!("sheets" / "ws" / String / String)
        .and(warp::ws())
        .and(editors)
        .map(
            |code: String, character: String, ws: warp::ws::Ws, editors| {
                ws.on_upgrade(move |socket| editor_connected(code, character, socket, editors))
            },
        );
    sock.or(character).or(index)
}

async fn editor_connected(
    code: String,
    character: String,
    ws: warp::ws::WebSocket,
    editors: Editors,
) {
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

    let place = format!("{}/{}", code, character);
    {
        // Save the sender in our list of connected users.
        let mut e = editors.write().await;
        if e.get(&place).is_none() {
            e.insert(place.clone(), Vec::new());
        }
        e.get_mut(&place).unwrap().push(tx);
        println!("got {} connections now", e.get(&place).unwrap().len());
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
        process_message(&code, &character, msg, &editors).await;
    }

    // user_ws_rx stream will keep processing as long as the user stays
    // connected. Once they disconnect, then...
    ws_disconnected(&editors2).await;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Change {
    kind: String,
    id: String,
    html: String,
    color: String,
}

async fn process_message(code: &str, character: &str, msg: warp::ws::Message, editors: &Editors) {
    let mut character = Character::read(code, character);
    let change: Change = serde_json::from_str(msg.to_str().expect("utf8")).expect("parsing sonj");
    character.change(&change);
    println!("character is {:?}", character);
    character.save();
    for tx in editors.read().await.get(code).iter().flat_map(|x| x.iter()) {
        println!("Sending {:?} to {:?}", msg, tx);
        if let Err(_disconnected) = tx.send(Ok(msg.clone())) {
            // The tx is disconnected, our `user_disconnected` code
            // should be happening in another task, nothing more to
            // do here.
            println!("Websocket disconnected?");
        }
    }
    // editors.write().await.get_mut(code).unwrap().retain(|x| !x.is_closed());
}

async fn ws_disconnected(_editors: &Editors) {}

struct Index;
#[with_template("[%" "%]" "index.html")]
impl DisplayAs<HTML> for Index {}
