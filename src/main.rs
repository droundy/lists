use warp::{Filter, path};
use display_as::{with_template, format_as, display, HTML, UTF8, URL, DisplayAs};
use serde_derive::{Deserialize, Serialize};

mod atomicfile;

fn main() {
    let style_css = path!("style.css").and(warp::fs::file("style.css"));
    let new = path!("new-thing")
        .and(warp::filters::body::form())
        .map(|change: NewThing| {
            println!("creating new thing {:?}", change);
            change.save();
            "okay"
        });
    let index = (warp::path::end().or(path!("index.html")))
        .map(|_| {
            display(HTML, &Thing::read()).http_response()
        });
    let list = path!("list" / String)
        .map(|parent: String| {
            if let Some(x) = Thing::read().lookup(&parent) {
                display(HTML, x).http_response()
            } else {
                display(HTML, &Thing::read()).http_response()
            }
        });

    let things = Thing::read();
    things.save();

    warp::serve(style_css
                .or(new)
                .or(list)
                .or(index))
        .run(([0, 0, 0, 0], 3000));
}

#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NewThing {
    name: String,
    parent: String,
}

impl NewThing {
    fn save(&self) {
        let mut things = Thing::read();
        if let Some(subthing) = things.lookup_mut(&self.parent) {
            subthing.children.push(Thing {
                name: self.name.clone(),
                times_used: 0,
                times_skipped: 0,
                children: Vec::new(),
            });
        }
        things.save();
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Thing {
    name: String,
    times_used: u64,
    times_skipped: u64,
    children: Vec<Thing>,
}

#[with_template("[%" "%]" "things.html")]
impl DisplayAs<HTML> for Thing {}
#[with_template(r#"<a href="/list/"# self.name r#"">"# self.name "</a>")]
impl DisplayAs<URL> for Thing {}

impl Thing {
    fn priority(&self) -> u64 {
        self.times_used + self.times_skipped
    }
    fn lookup(&self, parent: &str) -> Option<&Self> {
        if self.name == parent {
            return Some(self);
        }
        for child in self.children.iter() {
            if let Some(x) = child.lookup(parent) {
                return Some(x);
            }
        }
        None
    }
    fn lookup_mut(&mut self, parent: &str) -> Option<&mut Self> {
        if self.name == parent {
            return Some(self);
        }
        for child in self.children.iter_mut() {
            if let Some(x) = child.lookup_mut(parent) {
                return Some(x);
            }
        }
        None
    }
    fn read() -> Self {
        if let Ok(f) = ::std::fs::File::open("things.yaml") {
            if let Ok(s) = serde_yaml::from_reader::<_,Self>(&f) {
                return s;
            }
        }
        Thing {
            name: String::from("Everything"),
            times_used: 0,
            times_skipped: 0,
            children: Vec::new(),
        }
    }
    fn save(&self) {
        let f = atomicfile::AtomicFile::create("things.yaml")
            .expect("error creating save file");
        serde_yaml::to_writer(&f, self).expect("error writing yaml")
    }
}

impl PartialOrd for Thing {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Thing {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority().cmp(&other.priority())
    }
}
