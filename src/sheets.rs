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
    table_id: String,
    table: Vec<Row>,
}
#[with_template("[%" "%]" "section.html")]
impl DisplayAs<HTML> for Section {}

impl Section {
    fn change(&mut self, change: &Change) -> Option<Change> {
        if change.kind == "change" {
            if change.id == self.title_id {
                self.title = change.html.clone();
                return Some(change.clone());
            } else if change.id == self.content_id {
                self.content = change.html.clone();
                return Some(change.clone());
            }
        } else if change.kind == "new-row" && change.id == self.table_id {
            let r = Row {
                id: memorable_wordlist::camel_case(44),
                items: vec![Item {
                    id: memorable_wordlist::camel_case(44),
                    html: "_".to_string(),
                }],
            };
            println!("pusing new row!");
            self.table.push(r.clone());
            return Some(Change {
                kind: "new-row".to_string(),
                id: self.table_id.clone(),
                color: change.color.clone(),
                html: display_as::format_as!(HTML, r),
            });
        }
        for r in self.table.iter_mut() {
            if let Some(c) = r.change(change) {
                if change.kind == "del-item" && self.table.iter().any(|i| i.items.len() == 0) {
                    self.table.retain(|r| r.items.len() > 0);
                    return Some(Change {
                        kind: "replace".to_string(),
                        id: self.table_id.clone(),
                        html: display_as::format_as!(HTML, self),
                        color: change.color.clone(),
                    });
                }
                return Some(c);
            }
        }
        None
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Row {
    id: String,
    items: Vec<Item>,
}
impl Row {
    fn change(&mut self, change: &Change) -> Option<Change> {
        if change.kind == "change" && change.id == self.id {
            println!("this is odd!")
        }
        if change.kind == "new-item" && change.id == self.id {
            self.items.push(Item {
                id: memorable_wordlist::camel_case(44),
                html: "_".to_string(),
            });
            return Some(Change {
                kind: "replace".to_string(),
                id: self.id.clone(),
                html: display_as::format_as!(HTML, self),
                color: change.color.clone(),
            });
        }
        if change.kind == "del-item" && change.id == self.id {
            self.items.retain(|x| x.html.len() > 0);
            return Some(Change {
                kind: "replace".to_string(),
                id: self.id.clone(),
                html: display_as::format_as!(HTML, self),
                color: change.color.clone(),
            });
        }
        for i in self.items.iter_mut() {
            if change.id == i.id {
                i.html = change.html.clone();
                return Some(change.clone());
            }
        }
        None
    }
}
#[with_template("[%" "%]" "row.html")]
impl DisplayAs<HTML> for Row {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Item {
    id: String,
    html: String,
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
            sections: Vec::new(),
        }
    }
    fn save(&self) {
        std::fs::create_dir_all(format!("data/{}", self.code)).unwrap();
        let f = atomicfile::AtomicFile::create(format!("data/{}/{}.yaml", self.code, self.name))
            .expect("error creating save file");
        serde_yaml::to_writer(&f, self).expect("error writing yaml")
    }
    fn change(&mut self, change: &Change) -> Option<Change> {
        for c in self.sections.iter_mut() {
            if let Some(cnew) = c.change(&change) {
                return Some(cnew);
            }
        }
        if change.kind == "new-section" {
            let s = Section {
                title: "".to_string(),
                title_id: memorable_wordlist::camel_case(44),
                content: "".to_string(),
                content_id: memorable_wordlist::camel_case(44),
                table_id: memorable_wordlist::camel_case(44),
                table: Vec::new(),
            };
            self.sections.push(s.clone());
            return Some(Change {
                kind: "new-section".to_string(),
                id: "main".to_string(),
                html: display_as::format_as!(HTML, s),
                color: change.color.clone(),
            });
        }
        if change.kind == "change" {
            let s = Section {
                title: change.html.clone(),
                title_id: change.id.clone(),
                content: "Edit me".to_string(),
                content_id: memorable_wordlist::camel_case(44),
                table_id: memorable_wordlist::camel_case(44),
                table: Vec::new(),
            };
            self.sections.push(s.clone());
            return Some(Change {
                kind: "new-section".to_string(),
                id: "sections".to_string(),
                html: display_as::format_as!(HTML, s),
                color: change.color.clone(),
            });
        }
        None
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

    let css = path!("sheets" / "character.css")
        .or(path!("sheets" / String / "character.css"))
        .map(|_| {
            const STYLE: &'static str = include_str!("character.css");
            Ok(warp::http::Response::builder()
                .status(200)
                .header("content-length", STYLE.len())
                .header("content-type", "text/css")
                .body(STYLE)
                .unwrap())
        });
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
            display(HTML, &Character::read(&code, &character_name)).into_response()
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
                let character = percent_encoding::percent_decode(character.as_bytes())
                    .decode_utf8()
                    .unwrap()
                    .to_string();
                let code = percent_encoding::percent_decode(code.as_bytes())
                    .decode_utf8()
                    .unwrap()
                    .to_string();
                ws.on_upgrade(move |socket| editor_connected(code, character, socket, editors))
            },
        );
    sock.or(css).or(character).or(index)
}

async fn editor_connected(
    code: String,
    character: String,
    ws: warp::ws::WebSocket,
    editors: Editors,
) {
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

async fn process_message(
    code: &str,
    character: &str,
    mut msg: warp::ws::Message,
    editors: &Editors,
) {
    let place = format!("{}/{}", code, character);
    let mut character = Character::read(code, character);
    match msg.to_str().map(|s| serde_json::from_str(s)) {
        Err(e) => {
            eprintln!("Bad UTF8: {:?} {:?}", e, msg);
        }
        Ok(Err(e)) => {
            eprintln!("Bad JSON: {:?}", e);
        }
        Ok(Ok(change)) => {
            if let Some(newc) = character.change(&change) {
                msg = warp::ws::Message::text(serde_json::to_string(&newc).unwrap());
            }
            character.save();
            for tx in editors.read().await.get(&place).unwrap().iter() {
                if let Err(_disconnected) = tx.send(Ok(msg.clone())) {
                    // The tx is disconnected, our `user_disconnected` code
                    // should be happening in another task, nothing more to
                    // do here.
                    println!("Websocket disconnected?");
                }
            }
        }
    }
    // editors.write().await.get_mut(code).unwrap().retain(|x| !x.is_closed());
}

async fn ws_disconnected(_editors: &Editors) {}

struct Index;
#[with_template("[%" "%]" "sheet-index.html")]
impl DisplayAs<HTML> for Index {}
