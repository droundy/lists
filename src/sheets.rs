use crate::atomicfile;
use display_as::{display, with_template, DisplayAs, HTML, URL, UTF8};
use serde::{Deserialize, Serialize};
use warp::reply::Reply;
use warp::{path, Filter};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Character {
    name: String,
    spell_points: Option<(i32, i32)>,
    psi: Option<(i32, i32)>,
    sections: Vec<Section>,
}

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
    roll: String,
    base: String,
    speed: String,
    bonus: String,
    damage: String,
    level: String,
    cost: String,
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
    name: String,
    characters: Vec<Character>,
}

impl Party {
    fn read(code: &str, name: &str) -> Self {
        if let Ok(f) = ::std::fs::File::open(format!("data/{}/{}", code, name)) {
            if let Ok(mut s) = serde_yaml::from_reader::<_, Vec<Character>>(&f) {
                // Anything that has an empty name should just be deleted...
                s.retain(|x| x.name.len() > 0);
                return Party {
                    code: code.to_string(),
                    name: name.to_string(),
                    characters: s,
                };
            }
        }
        Party {
            code: code.to_string(),
            name: name.to_string(),
            characters: Vec::new(),
        }
    }
    fn save(&self) {
        let f = atomicfile::AtomicFile::create(format!("data/{}/{}", self.code, self.name))
            .expect("error creating save file");
        serde_yaml::to_writer(&f, &self.characters).expect("error writing yaml")
    }
}

pub fn sheets() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let index = path!("sheets").map(|| {
        println!("I am doing index.");
        display(HTML, &Index).into_response()
    });
    let list = path!("sheets" / String / String).map(|code: String, listname: String| {
        let listname = percent_encoding::percent_decode(listname.as_bytes())
            .decode_utf8()
            .unwrap();
        let code = percent_encoding::percent_decode(code.as_bytes())
            .decode_utf8()
            .unwrap();
        let x = Party::read(&code, &listname);
        display(HTML, &"goodbye").into_response()
    });
    let list_of_lists = path!("sheets" / String).map(|code: String| {
        println!("list of lists: {}", code);
        let code = percent_encoding::percent_decode(code.as_bytes())
            .decode_utf8()
            .unwrap();
        let x = Party::read(&code, "fixme");
        display(HTML, &"hello").into_response()
    });
    list.or(list_of_lists).or(index)
}

struct Index;
#[with_template("[%" "%]" "index.html")]
impl DisplayAs<HTML> for Index {}
