use warp::{Filter, path};
use display_as::{with_template, format_as, display, HTML, UTF8, URL, DisplayAs};
use serde_derive::{Deserialize, Serialize};

mod atomicfile;
mod time;

use time::{Time, now, DAY};

fn main() {
    let style_css = path!("style.css").and(warp::fs::file("style.css"));
    let js = path!("random-pass.js").and(warp::fs::file("random-pass.js"));
    let new = path!("new-thing")
        .and(warp::filters::body::form())
        .map(|change: NewThing| {
            println!("creating new thing {:?}", change);
            change.save();
            "okay"
        });
    let index = (warp::path::end().or(path!("index.html")))
        .map(|_| {
            display(HTML, &Index {}).http_response()
        });
    let list = path!(String / String)
        .map(|code: String, listname: String| {
            let x = ThingList::read(&code, &listname);
            display(HTML, &x).http_response()
        });
    let list_of_lists = path!(String)
        .map(|code: String| {
            let x = ThingList::read(&code, "fixme");
            display(HTML, &x).http_response()
        });

    warp::serve(style_css
                .or(js)
                .or(new)
                .or(list)
                .or(list_of_lists)
                .or(index))
        .run(([0, 0, 0, 0], 3000));
}

struct Index {}
#[with_template("[%" "%]" "index.html")]
impl DisplayAs<HTML> for Index {}

#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NewThing {
    code: String,
    name: String,
    list: String,
}

impl NewThing {
    fn save(&self) {
        let mut list = ThingList::read(&self.code, &self.list);
        let mut newthing = Thing {
            name: self.name.clone(),
            next: now(),
            first_chosen: now(),
            chosen: now(),
            created: now(),
            count: 0,
        };
        list.things.push(newthing);
        list.save();
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ChooseThing {
    code: String,
    name: String,
    list: String,
}

impl ChooseThing {
    fn save(&self) {
        let mut list = ThingList::read(&self.code, &self.list);
        let mut newthing = Thing {
            name: self.name.clone(),
            next: now(),
            first_chosen: now(),
            chosen: now(),
            created: now(),
            count: 0,
        };
        list.things.push(newthing);
        list.save();
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DelayThing {
    code: String,
    name: String,
    list: String,
}

#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ThingList {
    code: String,
    name: String,
    things: Vec<Thing>,
}

#[with_template("[%" "%]" "things.html")]
impl DisplayAs<HTML> for ThingList {}

#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Thing {
    name: String,
    created: Time,
    first_chosen: Time,
    chosen: Time,
    next: Time,
    count: u64,
}
#[with_template(r#"<a href="/list/"# self.name r#"">"# self.name "</a>")]
impl DisplayAs<URL> for Thing {}

impl Thing {
    fn priority(&self) -> Time {
        self.next
    }
    fn mean_interval(&self) -> Time {
        if self.count < 2 {
            DAY
        } else {
            (self.chosen - self.first_chosen) / (self.count - 1)
        }
    }
    fn choose(&mut self, mean_interval_all: Time) {
        // print(
        // 'choosing: ${prettyTime(chosen)}  and  ${prettyDuration(meanInterval)}  and  ${prettyDuration(meanIntervalList)}');
        let now = now();
        let last_interval = now - self.chosen;
        self.next = if self.count > 1 {
            now + time::geometric_mean(&[last_interval,
                                         self.mean_interval(),
                                         mean_interval_all])
        } else if self.count == 1 {
            now + time::geometric_mean(&[last_interval, mean_interval_all])
        } else {
            now + mean_interval_all
        };
        self.chosen = now;
        self.count += 1;
        if self.count == 1 {
            self.first_chosen = now;
        }
    }

}

fn read_lists(code: &str) -> Vec<String> {
    let dir: std::path::PathBuf = format!("data/{}", code).into();
    match std::fs::read_dir(&dir) {
        Ok(ddd) => {
            let mut lists = Vec::new();
            for entry in ddd {
                if let Ok(entry) = entry {
                    if let Some(s) = entry.path().to_str()
                        .iter().flat_map(|x| x.rsplit('/')).next() {
                        lists.push(s.to_string());
                    }
                }
            }
            lists
        }
        Err(_) => {
            Vec::new()
        }
    }
}

impl ThingList {
    fn read(code: &str, name: &str) -> Self {
        if let Ok(f) = ::std::fs::File::open(format!("data/{}/{}", code, name)) {
            if let Ok(s) = serde_yaml::from_reader::<_,Vec<Thing>>(&f) {
                return ThingList {
                    code: code.to_string(),
                    name: name.to_string(),
                    things: s,
                };
            }
        }
        ThingList {
            code: code.to_string(),
            name: name.to_string(),
            things: Vec::new(),
        }
    }
    fn save(&self) {
        let f = atomicfile::AtomicFile::create(format!("data/{}/{}",
                                                       self.code, self.name))
            .expect("error creating save file");
        serde_yaml::to_writer(&f, &self.things).expect("error writing yaml")
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
