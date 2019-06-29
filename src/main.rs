use warp::{Filter, path};
use display_as::{with_template, display, HTML, URL, UTF8, DisplayAs};
use serde::{Deserialize, Serialize};
use clapme::ClapMe;

mod atomicfile;

#[derive(Debug, ClapMe, Serialize)]
struct Flags {
    /// Port to serve on, if not port 80.
    port: Option<u16>,
}

fn percent_decode(x: &str) -> String {
    percent_encoding::percent_decode(x.as_bytes()).decode_utf8().unwrap().to_string()
}

fn main() {
    let flags = Flags::from_args();
    let style_css = path!("style.css").and(warp::fs::file("style.css"));
    let edit = path!("edit-thing")
        .and(warp::filters::body::form())
        .map(|change: EditThing| {
            println!("creating new thing {:?}", change);
            change.edit();
            "okay"
        });
    let new = path!("new-thing")
        .and(warp::filters::body::form())
        .map(|change: NewThing| {
            println!("creating new thing {:?}", change);
            change.save();
            "okay"
        });
    let choose = path!("choose-thing")
        .and(warp::filters::body::form())
        .map(|change: ChooseThing| {
            println!("choosing thing {:?}", change);
            display(HTML, &change.choose()).http_response()
        });
    let delay = path!("pass" / String / String / String)
        .map(|code: String, list: String, name: String| {
            let change = ChooseThing {
                code: percent_decode(&code),
                name: percent_decode(&name),
                list: percent_decode(&list),
            };
            println!("delay thing {:?}", change);
            display(HTML, &change.delay()).http_response()
        });
    let index = (warp::path::end().or(path!("index.html")))
        .map(|_| {
            display(HTML, &Index {}).http_response()
        });
    let search = path!("search" / String / String / String)
        .map(|code: String, listname: String, pattern: String| {
            let listname = percent_encoding::percent_decode(listname.as_bytes())
                .decode_utf8().unwrap();
            let code = percent_encoding::percent_decode(code.as_bytes())
                .decode_utf8().unwrap();
            let pattern = if pattern != "qqq" {
                percent_encoding::percent_decode(pattern.as_bytes())
                    .decode_utf8().unwrap()
            } else {
                "".into()
            };
            let x = ThingsOnly(ThingList::read(&code, &listname).filter(&pattern));
            display(HTML, &x).http_response()
        });
    let sort = path!("sort" / String / String)
        .map(|code: String, listname: String| {
            println!("I am sorting the list.");
            let listname = percent_encoding::percent_decode(listname.as_bytes())
                .decode_utf8().unwrap();
            let code = percent_encoding::percent_decode(code.as_bytes())
                .decode_utf8().unwrap();
            let x = ThingsOnly(ThingList::read(&code, &listname).sorted());
            println!("I am done sorting the list.");
            display(HTML, &x).http_response()
        });
    let list = path!(String / String)
        .map(|code: String, listname: String| {
            let listname = percent_encoding::percent_decode(listname.as_bytes())
                .decode_utf8().unwrap();
            let code = percent_encoding::percent_decode(code.as_bytes())
                .decode_utf8().unwrap();
            let x = ThingList::read(&code, &listname);
            display(HTML, &x).http_response()
        });
    let list_of_lists = path!(String)
        .map(|code: String| {
            let code = percent_encoding::percent_decode(code.as_bytes())
                .decode_utf8().unwrap();
            let x = ThingList::read(&code, "fixme");
            display(HTML, &x).http_response()
        });

    warp::serve(style_css
                .or(edit)
                .or(new)
                .or(choose)
                .or(delay)
                .or(sort)
                .or(search)
                .or(list)
                .or(list_of_lists)
                .or(index))
        .run(([0, 0, 0, 0], flags.port.unwrap_or(80)));
}

struct Index {}
#[with_template("[%" "%]" "index.html")]
impl DisplayAs<HTML> for Index {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NewThing {
    code: String,
    name: String,
    list: String,
}

impl NewThing {
    fn save(&self) {
        let mut list = ThingList::read(&self.code, &self.list);
        let now = list.now();
        let newthing = Thing {
            name: self.name.clone(),
            link: None,
            next: now,
            first_chosen: now,
            chosen: now,
            created: now,
            count: 0,
            parent_code: self.code.clone(),
            parent_name: self.list.clone(),
        };
        list.things.push(newthing);
        list.save();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ChooseThing {
    code: String,
    name: String,
    list: String,
}

impl ChooseThing {
    fn choose(&self) -> ThingsOnly {
        let mut list = ThingList::read(&self.code, &self.list);
        list.choose(&self.name);
        list.save();
        ThingsOnly(list)
    }
    fn delay(&self) -> ThingsOnly {
        let mut list = ThingList::read(&self.code, &self.list);
        list.delay(&self.name);
        list.save();
        ThingsOnly(list)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct EditThing {
    code: String,
    name: String,
    list: String,
    link: String,
    newname: String,
}

impl EditThing {
    fn edit(&self) {
        let mut list = ThingList::read(&self.code, &self.list);
        let th = list.edit(&self.name);
        th.name = self.newname.clone();
        if self.link.len() > 0 {
            th.link = Some(self.link.clone());
        }
        list.save();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Thing {
    name: String,
    #[serde(default)]
    link: Option<String>,
    created: f64,
    first_chosen: f64,
    chosen: f64,
    next: f64,
    count: u64,
    parent_code: String,
    parent_name: String,
}
#[with_template("[%" "%]" "thing.html")]
impl DisplayAs<HTML> for Thing {}

impl Thing {
    fn slug(&self) -> String {
        self.name.replace("'", "-")
            .replace(" ", "-")
            .replace("\"", "-")
            .replace("\\", "-")
    }
    fn priority(&self) -> f64 {
        self.next
    }
    fn mean_interval(&self) -> f64 {
        if self.count < 2 {
            1.0
        } else {
            (self.chosen - self.first_chosen) / (self.count as f64 - 1.0)
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ThingList {
    code: String,
    name: String,
    things: Vec<Thing>,
}

#[with_template("[%" "%]" "things.html")]
impl DisplayAs<HTML> for ThingList {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ThingsOnly(ThingList);

#[with_template("[%" "%]" "things-only.html")]
impl DisplayAs<HTML> for ThingsOnly {}

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
    fn filter(mut self, s: &str) -> Self {
        self.things.retain(|x| x.name.contains(s));
        self
    }
    fn sorted(mut self) -> Self {
        self.things.sort_by_key(|x| x.chosen as i64);
        self.things.sort_by_key(|x| x.count as i64);
        let now = self.now();
        for (i,x) in self.things.iter_mut().enumerate() {
            x.next = now + i as f64;
        }
        self.save();
        self
    }
    fn now(&self) -> f64 {
        self.things.iter().map(|x| x.count as f64).sum()
    }
    fn mean_interval(&self) -> f64 {
        (self.now()+1.0)/((self.things.len()+1) as f64)
    }
    fn save(&self) {
        let f = atomicfile::AtomicFile::create(format!("data/{}/{}",
                                                       self.code, self.name))
            .expect("error creating save file");
        serde_yaml::to_writer(&f, &self.things).expect("error writing yaml")
    }
    fn edit(&mut self, which: &str) -> &mut Thing {
        let mut wh = self.things.len();
        for (i,th) in self.things.iter().enumerate() {
            if th.name == which {
                wh = i;
                break;
            }
        }
        if wh == self.things.len() {
            let now = self.now() + 1.0;
            self.things.push(Thing {
                name: which.to_string(),
                link: None,
                next: now,
                first_chosen: now,
                chosen: now,
                created: now,
                count: 0,
                parent_name: self.name.clone(),
                parent_code: self.code.clone(),
            });
        }
        &mut self.things[wh]
    }
    fn choose(&mut self, which: &str) {
        // print(
        // 'choosing: ${prettyTime(chosen)}  and  ${prettyDuration(meanInterval)}  and  ${prettyDuration(meanIntervalList)}');
        let now = self.now() + 1.0;
        let list_mean = self.mean_interval();
        let mut which_num = 0;
        let mut thing = Thing {
            name: which.to_string(),
            link: None,
            next: now,
            first_chosen: now,
            chosen: now,
            created: now,
            count: 0,
            parent_name: self.name.clone(),
            parent_code: self.code.clone(),
        };
        for (i,th) in self.things.iter_mut().enumerate() {
            if th.name == which {
                thing = th.clone();
                which_num = i;
                let last_interval = now - thing.chosen;
                thing.next = if thing.count > 1 {
                    now + geometric_mean(&[last_interval,
                                           thing.mean_interval(),
                                           list_mean])
                } else if thing.count == 1 {
                    now + geometric_mean(&[last_interval, list_mean])
                } else {
                    now + list_mean
                };
                thing.chosen = now;
                thing.count += 1;
                if thing.count == 1 {
                    thing.first_chosen = now;
                }
            }
        }
        self.things.remove(which_num);
        let mut place = 0;
        for (i,th) in self.things.iter().enumerate() {
            if th.next <= thing.next {
                place = i+1;
            }
        }
        self.things.insert(place, thing);
    }
    fn delay(&mut self, which: &str) {
        // print(
        // 'choosing: ${prettyTime(chosen)}  and  ${prettyDuration(meanInterval)}  and  ${prettyDuration(meanIntervalList)}');
        let now = self.now() + 1.0;
        let list_mean = self.mean_interval();
        let mut which_num = 0;
        let mut thing = Thing {
            name: self.name.clone(),
            link: None,
            next: now,
            first_chosen: now,
            chosen: now,
            created: now,
            count: 0,
            parent_name: self.name.clone(),
            parent_code: self.code.clone(),
        };

        for (i,th) in self.things.iter_mut().enumerate() {
            if th.name == which {
                thing = th.clone();
                which_num = i;
            }
        }
        if which_num == self.things.len() - 1 {
            // It is already last, no point delaying!
            return;
        }
        while self.things[which_num].name == which {
            // Checking that the thing was actually found and hasn't yet moved
            self.things.remove(which_num);
            thing.next += (list_mean+1.0)*0.5;
            let mut place = 0;
            for (i,th) in self.things.iter().enumerate() {
                if th.next <= thing.next {
                    place = i+1;
                }
            }
            self.things.insert(place, thing.clone());
        }
    }
}

impl PartialOrd for Thing {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Thing {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.priority() < other.priority() {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    }
}

impl PartialEq for Thing {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Thing {}

pub fn geometric_mean(data: &[f64]) -> f64 {
    let mut product = 1.0;
    for &d in data.iter() {
        product *= d;
    }
    product.powf(1.0/data.len() as f64)
}
